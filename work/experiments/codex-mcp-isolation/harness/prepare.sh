#!/bin/bash
# Prepare a work directory: a throwaway test CA with a server certificate for 127.0.0.1, and the stand-in bundle.
# usage: prepare.sh <work-dir>
set -eu
H=$(cd "$(dirname "$0")" && pwd); mkdir -p "$1/certs" "$1/results"; WORK=$(cd "$1" && pwd)
cd "$WORK/certs"
printf 'basicConstraints = critical, CA:FALSE\nkeyUsage = critical, digitalSignature, keyEncipherment\nextendedKeyUsage = serverAuth\nsubjectAltName = IP:127.0.0.1, DNS:localhost\n' > srv.ext
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem -days 3 -subj "/CN=codex-mcp-isolation test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" 2>/dev/null
openssl req -newkey rsa:2048 -nodes -keyout srv.key -out srv.csr -subj "/CN=127.0.0.1" 2>/dev/null
openssl x509 -req -in srv.csr -CA ca.pem -CAkey ca.key -CAcreateserial -out srv.pem -days 3 -extfile srv.ext 2>/dev/null
openssl verify -CAfile ca.pem srv.pem
python3 "$H/build_bundle.py" "$WORK/bundle.tar.gz" "$WORK/markers.jsonl"
