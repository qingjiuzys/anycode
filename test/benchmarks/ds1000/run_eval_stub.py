"""Minimal benchmark stub — real harness mounts dataset at /data."""
import json, os, sys
out = os.environ.get("OUT", "/out")
os.makedirs(out, exist_ok=True)
json.dump({"adapter": "ds1000", "status": "stub_executed"}, open(f"{out}/result.json", "w"))
print("DS-1000 stub ok")
