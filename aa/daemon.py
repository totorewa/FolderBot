#!/usr/bin/env python3

# Where we store stuff for the daemon.
DATA_DIR: None | str = None


def data_dir():
    # one-time initialization
    global DATA_DIR
    if DATA_DIR is not None:
        return DATA_DIR

    from os import environ, makedirs
    from os.path import expanduser, isdir, isfile, join
    state = environ.get("XDG_DATA_HOME", expanduser("~/.local/share"))
    if isfile(state):
        # idk, handle this error just in the off chance it happens? lol
        raise RuntimeError(f"Data/share folder ({state}) is a file -- don't do this.")
    full_data = join(state, "paceman-data")
    assert not isfile(full_data)
    if not isdir(full_data):
        makedirs(full_data, exist_ok=True)
    DATA_DIR = full_data
    return DATA_DIR


def localfile(s):
    from os.path import join, dirname
    return join(dirname(__file__), s)


def datafile(s):
    from os.path import join, dirname
    return join(data_dir(), s)


def get_days(n: int):
    # this function may be a problem
    from requests import get 
    EVIL_URL_DO_NOT_USE = open(localfile("bad")).read().strip().format(DAYS=n)
    print(f"Making request to: {EVIL_URL_DO_NOT_USE}")
    result = get(EVIL_URL_DO_NOT_USE)
    return result.json()["data"]


class UpdatedToken:
    def __init__(self, did_update: bool = True):
        if not did_update:
            return
        update_date = str(UpdatedToken.current_seconds())
        with open(UpdatedToken.filename(), "w") as file:
            file.write(update_date)

    @staticmethod
    def filename():
        return datafile("last_update")

    @staticmethod
    def current_seconds():
        from time import time
        return int(time())


def get_id(g):
    return g["id"]

def get_key(k):
    def getter(d):
        return d[k]
    return getter

# 2024-12-27T13:11:14.000Z
DATE_FORMAT = "%Y-%m-%dT%H:%M:%S"
def as_datetime(ts: str):
    from datetime import datetime, timezone
    return datetime.strptime(ts[:-5], DATE_FORMAT).replace(tzinfo=timezone.utc)


def as_seconds(ts: str):
    return as_datetime(ts).timestamp()


def duration_since(ts: str):
    from datetime import datetime, timezone
    return datetime.now(timezone.utc) - as_datetime(ts)


def time_since(ts: str):
    return str(duration_since(ts))

def seconds_since_update():
    now = UpdatedToken.current_seconds()
    then = int(open(UpdatedToken.filename()).read().strip())
    return abs(now - then)

def merge(newest, others):
    has = set([get_id(d) for d in newest])
    return newest + [o for o in others if get_id(o) not in has]

def mode(to_set: str | None = None):
    fn = datafile("enabled")
    if to_set is not None:
        open(fn, 'w').write(to_set)
    return open(fn, 'r').read().strip()


def auto_update(filename: str = "aa.json", source_uri: str = "bad") -> UpdatedToken:
    from os.path import isfile
    import json

    if mode() == 'off':
        print('No update - disabled.')
        return UpdatedToken(False)
    
    # Get the URI for all API requests - hidden from source for now.
    if not source_uri.startswith("http"):
        source_uri = open(localfile(source_uri)).read().strip()

    # Get the actual data
    file = datafile(filename)
    if not isfile(file):
        # Simple, just get all the data. xd!
        all_data = get_days(9999)
        print("Got data -", len(all_data), "objects.")
        with open(file, 'w') as fp:
            json.dump(obj=all_data, fp=fp, indent=2)
        return UpdatedToken()

    # If we have the data, we must have an updated token.
    if seconds_since_update() < 10:
        return UpdatedToken(False)
    
    # Okay, we need to figure out how many more days we need.
    with open(file, 'r') as fp:
        # Cool
        data = sorted(json.load(fp), key=get_id)
    # Insanely optimistic to assume that we couldn't do this with 20.
    # But it doesn't matter. Basically irrelevant.
    last_50 = data[-50:]
    latest = min(last_50, key=lambda d: duration_since(d["lastUpdated"]).total_seconds())
    last_update = duration_since(latest["lastUpdated"])
    print("Latest known update:", last_update, f"({last_update.days} day(s))")
    required_days = last_update.days + 1

    new_data = get_days(required_days)
    full_data = merge(new_data, data)
    print(f"Updated to {len(full_data)} runs from {len(data)} (pulled {required_days} day(s) of data).")
    with open(file, 'w') as fp:
        json.dump(obj=full_data, fp=fp, indent=2)

    return UpdatedToken()


def main(args: list = list()):
    if 'off' in args:
        mode("off")
    elif 'on' in args:
        mode("on")
    else:
        auto_update()


if __name__ == "__main__":
    from sys import argv
    main(argv[1:])
