"""Independent Fraction boundary corpus, only a verification input generator."""
from fractions import Fraction as F
import json
from pathlib import Path
import sys

data = json.loads(Path(sys.argv[1]).read_text())
scale = data["track"]["timescale"]
movie_scale = data["summary"]["movieTimescale"]
end = F(int(data["summary"]["movieEnd"]["numerator"]), int(data["summary"]["movieEnd"]["denominator"]))
points = {F(-1, 10), F(0), end, end + F(1, 10)}
raw = sorted(F(int(s["pts"]), scale) for s in data["samples"])
raw.append(F(int(data["summary"]["rawSpan"][1]), scale))
edits = data["edits"] or [{"movieStart": 0, "movieDuration": end * movie_scale, "mediaTime": 0, "rate1616": 65536}]
for edit in edits:
    start = F(int(edit["movieStart"]), movie_scale)
    finish = start + F(edit["movieDuration"]) / movie_scale
    points.update((start, finish, (start + finish) / 2))
    if edit["mediaTime"] is None or not edit["rate1616"]:
        continue
    media, rate = F(int(edit["mediaTime"]), scale), F(edit["rate1616"], 65536)
    mapped = [start + (p - media) / rate for p in raw]
    inside = sorted({start, finish, *(p for p in mapped if start <= p <= finish)})
    points.update(inside)
    points.update((a + b) / 2 for a, b in zip(inside, inside[1:]))
requests = [{"method": "selectVideo", "body": {"track": data["ordinal"],
             "time": {"numerator": str(p.numerator), "denominator": str(p.denominator)}, "side": side}}
            for p in sorted(points) for side in ("before", "after")]
print(json.dumps(requests))
