"""Independent fractions oracle for browser metadata mapping; no pixel claims."""
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location("time_oracle", Path(__file__).with_name("video-time-check.py"))
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
wide = {"dts", "pts", "decodeStart", "rawStart", "rawEnd", "movieStart", "movieDuration", "mediaTime", "declaredTrackDuration"}


def numbers(value, key=None):
    if isinstance(value, dict):
        return {k:numbers(v,k) for k,v in value.items()}
    if isinstance(value, list):
        return [numbers(v) for v in value]
    return int(value) if key in wide and isinstance(value,str) else value


oracle.verify_mapping(numbers(json.loads(Path(sys.argv[1]).read_text())), decoded_frames=False)
