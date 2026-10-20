"""Write a fake ChatGPT sign-in (unsigned JWT, dummy tokens) into a throwaway CODEX_HOME."""
import base64, json, sys
from datetime import datetime, timezone
from pathlib import Path
b64 = lambda d: base64.urlsafe_b64encode(json.dumps(d).encode()).rstrip(b"=").decode()
claims = {"email": "repro@example.invalid", "https://api.openai.com/auth": {
    "chatgpt_plan_type": "pro", "chatgpt_user_id": "user-123", "chatgpt_account_id": "account-123"}}
jwt = f"{b64({'alg': 'none', 'typ': 'JWT'})}.{b64(claims)}.sig"
auth = {"auth_mode": "chatgpt", "OPENAI_API_KEY": None,
        "tokens": {"id_token": jwt, "access_token": "chatgpt-token", "refresh_token": "refresh-token", "account_id": "account-123"},
        "last_refresh": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")}
Path(sys.argv[1], "auth.json").write_text(json.dumps(auth, indent=2))
