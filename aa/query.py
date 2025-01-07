#!/usr/bin/env python3
from daemon import auto_update, datafile
import json
from collections import defaultdict

DATAFILE = "aa.json"
AA_TIME_BARRIER = (1 * 60 + 30) * 1000

def DATA() -> list[dict]:
    auto_update()
    data = json.load(open(datafile(DATAFILE)))
    return [o for o in data if o.get("nether") > AA_TIME_BARRIER]

def unique_keys():
    d = defaultdict(lambda *args: 0)
    for o in DATA():
        for k in o.keys():
            d[k] += 1
    return d

def all_values(key: str, postprocess=None, paired=False, pred=None):
    values = [(o[key], o) for o in DATA() if key in o]
    if pred is not None:
        values = [(val, o) for val, o in values if pred(o)]
    if postprocess is not None:
        values = [(postprocess(val), o) for val, o in values]
    if not paired:
        values = [val for val, _ in values]
    return values

def average_by(key: str, by: str):
    from datetime import timedelta
    d = defaultdict(lambda: list())
    for o in DATA():
        if key not in o:
            continue
        v = o[key]
        k = o[by]
        d[k].append(v)
    after = list()
    for k, v in d.items():
        after.append((k, len(v), sum(v) / len(v)))
    after = sorted(after, key=lambda o: o[2])
    for name, played, avg in after:
        print("Player:", name, "-", str(timedelta(milliseconds=int(avg))), f"({played})")


def pretty_with(key: str, sort=None, **kwargs):
    values = all_values(key, **kwargs, paired=True)
    res = list()
    for val, o in values:
        as_json = json.dumps(o, indent=2)
        res.append((f"{key}: {val}\n{as_json}", val))
    if sort:
        res = sorted(res, key=lambda v: v[1])
    return "\n\n".join([s for s, _ in res])

def pretty_ms(v: int):
    from datetime import timedelta
    return str(timedelta(milliseconds=v))

def main(args: list[str]):
    print(unique_keys())
    print(pretty_with("finish", postprocess=pretty_ms))
    average_by("nether", "nickname")
    # print(pretty_with("nether", pred=lambda o: o["nickname"] == "DesktopFolder", sort=True, postprocess=pretty_ms))

if __name__ == "__main__":
    from sys import argv
    main(argv[1:])
