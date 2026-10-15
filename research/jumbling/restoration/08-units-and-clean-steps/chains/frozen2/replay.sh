#!/bin/bash
ulimit -v 7000000
cd <scratch>/final/chains/frozen2
nice -n 8 python3 -I <scratch>/gen/verify_u.py <research/jumbling> <echain> chain.json replay.jsonl > replay.log 2>&1
touch replay.done
