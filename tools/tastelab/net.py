"""Serial GETs on adapter hosts, without environment proxies or credentials."""
from __future__ import annotations

import math
import time
from dataclasses import dataclass, field
from datetime import timezone
from email.utils import parsedate_to_datetime
from urllib.error import HTTPError, URLError
from urllib.parse import urljoin, urlsplit
from urllib.request import HTTPRedirectHandler, ProxyHandler, Request, build_opener

from tastelab import common

MAX_REDIRECTS = 3
MAX_WAIT = 120.0
RETRIES = 3
TIMEOUT = 30.0


class NetError(Exception):
    pass


@dataclass
class Response:
    url: str
    status: int
    headers: dict = field(default_factory=dict)
    body: bytes = b""


def host_allowed(host, patterns) -> bool:
    host = host.lower()
    return any(host == p.lower() if not p.startswith("*.") else host.endswith(p[1:].lower())
               for p in patterns)


def _host(url, hosts):
    # http.client sends only ASCII; a raw non-ASCII URL from a source must fail as one request, not the run.
    if not isinstance(url, str) or not url.isascii() or any(c.isspace() or ord(c) < 32 or ord(c) == 127 for c in url):
        raise NetError("invalid URL")
    try:
        parts = urlsplit(url)
        if (parts.scheme != "https" or not parts.hostname or ":" in parts.netloc or "\\" in parts.netloc
                or parts.username is not None or parts.password is not None or not hosts
                or not host_allowed(parts.hostname, hosts)):
            raise NetError("URL is outside the adapter's HTTPS hosts")
        return parts.hostname
    except ValueError:
        raise NetError("invalid URL") from None


class _NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def urllib_transport(url, headers, max_bytes, timeout) -> Response:
    # build_opener's default handlers contain no auth, netrc or cookie handler.
    opener = build_opener(ProxyHandler({}), _NoRedirect())
    request = Request(url, headers=headers, method="GET")
    try:
        try:
            response = opener.open(request, timeout=timeout)
        except HTTPError as exc:
            response = exc
        with response:
            response_headers = {k.lower(): v for k, v in response.headers.items()}
            length = response_headers.get("content-length")
            if length is not None:
                try:
                    if int(length) > max_bytes:
                        raise NetError("response exceeds the size cap")
                except ValueError:
                    raise NetError("invalid Content-Length") from None
            body = response.read(max_bytes + 1)
            if len(body) > max_bytes:
                raise NetError("response exceeds the size cap")
            return Response(url, response.code, response_headers, body)
    except (URLError, OSError) as exc:
        raise NetError("GET failed") from exc


class ReplayTransport:
    """Responses by URL, or lists of successive responses for retry tests."""
    def __init__(self, responses):
        self.responses = dict(responses)
        self.calls = []

    def __call__(self, url, headers, max_bytes, timeout):
        self.calls.append((url, dict(headers)))
        if url not in self.responses:
            raise NetError("unrecorded GET in replay")
        response = self.responses[url]
        if isinstance(response, list):
            if not response:
                raise NetError("exhausted replay")
            response = response.pop(0)
        if isinstance(response, Exception):
            raise response
        if isinstance(response, Response):
            return response
        status, response_headers, body = response
        return Response(url, status, response_headers, body)


class Client:
    def __init__(self, transport=None, *, sleep=time.sleep, clock=time.monotonic, wall_clock=time.time):
        self.transport = transport or urllib_transport
        self.sleep, self.clock, self.wall_clock = sleep, clock, wall_clock
        self._last = {}

    def _retry_wait(self, response, attempt):
        value = response.headers.get("retry-after") if response is not None else None
        if value is None:
            return min(2 ** attempt, MAX_WAIT)
        try:
            seconds = float(value)
        except ValueError:
            try:
                moment = parsedate_to_datetime(value)
                if moment.tzinfo is None:
                    moment = moment.replace(tzinfo=timezone.utc)
                seconds = moment.timestamp() - self.wall_clock()
            except (ValueError, TypeError, OverflowError):
                raise NetError("invalid Retry-After") from None
        if not math.isfinite(seconds) or seconds > MAX_WAIT:
            raise NetError("Retry-After exceeds the bounded wait")
        return max(0, seconds)

    def get(self, url, *, hosts, min_interval, max_bytes, accept="*/*", extra_headers=None) -> Response:
        if not math.isfinite(min_interval) or min_interval < 0 or type(max_bytes) is not int or max_bytes <= 0:
            raise ValueError("invalid interval or size cap")
        headers = {"User-Agent": common.USER_AGENT, "Accept": accept}
        for key, value in (extra_headers or {}).items():
            if key.lower() != "aic-user-agent" or value != common.USER_AGENT:
                raise NetError("only the source's AIC-User-Agent header may be added")
            headers[key] = value
        redirects, attempt = 0, 0
        while True:
            host = _host(url, hosts)
            # Share the strictest interval across every request of this adapter,
            # including cross-host redirects and its API/image downloads.
            key = tuple(sorted(hosts))
            remaining = min_interval - (self.clock() - self._last.get(key, -math.inf))
            if remaining > 0:
                self.sleep(remaining)
            self._last[key] = self.clock()
            response = None
            try:
                response = self.transport(url, headers, max_bytes, TIMEOUT)
                response.headers = {k.lower(): v for k, v in response.headers.items()}
                if response.url != url:
                    raise NetError("transport followed an unchecked redirect")
                if len(response.body) > max_bytes:
                    raise NetError("response exceeds the size cap")
            except NetError:
                if attempt == RETRIES:
                    raise
                self.sleep(self._retry_wait(None, attempt))
                attempt += 1
                continue
            if response.status in (301, 302, 303, 307, 308):
                if redirects >= MAX_REDIRECTS or not response.headers.get("location"):
                    raise NetError("redirect limit or missing Location")
                try:
                    url = urljoin(url, response.headers["location"])
                except ValueError:
                    raise NetError("invalid redirect Location") from None
                _host(url, hosts)
                redirects += 1
                continue
            if response.status == 429 or 500 <= response.status <= 599:
                if attempt == RETRIES:
                    raise NetError(f"GET retries exhausted ({response.status})")
                self.sleep(self._retry_wait(response, attempt))
                attempt += 1
                continue
            if response.status != 200:
                raise NetError(f"GET refused ({response.status})")
            return response
