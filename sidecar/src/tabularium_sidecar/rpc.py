# SPDX-License-Identifier: AGPL-3.0-only
import sys, json, traceback
from tabularium_sidecar.handlers import dispatch

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line: continue
        req = {}
        try:
            req = json.loads(line)
            resp = {"id": req.get("id"), "result": dispatch(req["method"], req.get("params", {}))}
        except Exception as e:
            resp = {"id": req.get("id"), "error": {"message": str(e), "trace": traceback.format_exc()}}
        sys.stdout.write(json.dumps(resp, ensure_ascii=False) + chr(10))
        sys.stdout.flush()

if __name__ == "__main__": main()
