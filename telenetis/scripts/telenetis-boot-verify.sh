#!/usr/bin/env bash
#
# telenetis-boot-verify.sh — post-deploy smoke check for the telenetis server
# (port 9800 by default). Verifies the five surfaces a production deploy must
# serve: /health, /api/snapshot, /ws (WebSocket), /events (SSE), /webhook.
#
# Usage:
#   ./scripts/telenetis-boot-verify.sh [BASE_URL]   # default http://127.0.0.1:9800
#
# Exit 0 on pass, non-zero on the first failure. Requires curl.

set -u
BASE="${1:-http://127.0.0.1:9800}"
BASE="${BASE%/}"
PASS=0
FAIL=0

req() {
  local desc="$1" code
  shift
  code="$("$@" -o /dev/null -s -w '%{http_code}' --max-time 10)"
  if [ "$code" = "200" ]; then
    echo "ok   $desc (200)"
    PASS=$((PASS + 1))
  else
    echo "FAIL $desc ($code)"
    FAIL=$((FAIL + 1))
  fi
}

echo "== telenetis boot-verify @ $BASE =="

req "GET  /health"            curl "$BASE/health"
req "GET  /api/snapshot?lang=en" curl "$BASE/api/snapshot?lang=en"
req "GET  /api/status"        curl "$BASE/api/status"
req "GET  /api/live/config"   curl "$BASE/api/live/config"

# POST /webhook with a minimal (unknown-kind) Telegram update — classifies to
# Unknown and still returns the canonical "ok" (200). A malformed JSON body is
# rejected 400 by the Json<Value> extractor, so the body MUST be valid JSON.
# When TELENETIS_WEBHOOK_SECRET is set on the server, echo it back in the
# X-Telegram-Bot-Api-Secret-Token header (else the server rejects with 403).
echo -n "POST /webhook (minimal update) ... "
BODY='{"update_id":0}'
CURL_POST=(curl -o /dev/null -s -w '%{http_code}' --max-time 10 \
  -H 'Content-Type: application/json' -d "$BODY")
if [ -n "${TELENETIS_WEBHOOK_SECRET:-}" ]; then
  CURL_POST+=(-H "X-Telegram-Bot-Api-Secret-Token: $TELENETIS_WEBHOOK_SECRET")
fi
CODE="$("${CURL_POST[@]}" "$BASE/webhook")"
if [ "$CODE" = "200" ]; then
  echo "ok (200)"; PASS=$((PASS + 1))
else
  echo "FAIL ($CODE)"; FAIL=$((FAIL + 1))
fi

# WebSocket: a 101 Upgrade from /ws proves the WS surface is alive.
echo -n "WS   /ws  (upgrade) .......... "
CODE="$(curl -o /dev/null -s -w '%{http_code}' --max-time 10 \
  -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
  -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
  -H 'Sec-WebSocket-Version: 13' "$BASE/ws")"
if [ "$CODE" = "101" ]; then
  echo "ok (101)"; PASS=$((PASS + 1))
else
  echo "FAIL ($CODE)"; FAIL=$((FAIL + 1))
fi

# SSE: /events must stream text/event-stream.
echo -n "SSE  /events (headers) ....... "
CT="$(curl -o /dev/null -s --max-time 4 \
  -H 'Accept: text/event-stream' \
  -D - "$BASE/events" | grep -i '^content-type:' | tr -d '\r')"
case "$CT" in
  *text/event-stream*) echo "ok ($CT)"; PASS=$((PASS + 1)) ;;
  *) echo "FAIL ($CT)"; FAIL=$((FAIL + 1)) ;;
esac

echo
echo "== result: $PASS ok, $FAIL fail =="
[ "$FAIL" -eq 0 ]
