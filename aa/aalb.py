import asyncio
import json
import logging
import os
import time
from typing import List, Dict
import requests


class RoroConfig:
    def __init__(self, path: str = ".roroapi.json"):
        try:
            with open(path) as f:
                data = json.load(f)
        except FileNotFoundError:
            with open(os.path.join("../", path)) as f:
                data = json.load(f)
        self.base_url = data["base_url"]
        self.client_id = data["client_id"]
        self.client_secret = data["client_secret"]


class AALeaderboardAPI:
    def __init__(self, config: RoroConfig):
        self.base_url = f"{config.base_url}/api/leaderboard"
        self.auth = (config.client_id, config.client_secret)

    async def get_boards(self) -> List[Dict]:
        return await asyncio.to_thread(self._get_boards_sync)

    def _get_boards_sync(self) -> List[Dict]:
        url = f"{self.base_url}/boards"
        params = {"cat": "aa"}
        resp = requests.get(url, params=params, auth=self.auth)
        resp.raise_for_status()
        return resp.json()

    async def search(self, board: str, params: Dict) -> List[Dict]:
        logging.debug(f"Querying {board} with params: {params}")
        return await asyncio.to_thread(self._search_sync, board, params)

    def _search_sync(self, board: str, params: Dict) -> List[Dict]:
        url = f"{self.base_url}/search"
        params.update({"cat": "aa", "board": board})
        resp = requests.get(url, params=params, auth=self.auth)
        resp.raise_for_status()
        data = resp.json()
        return data.get("results", [])

    async def get_total_records(self, board: str) -> int:
        return await asyncio.to_thread(self._get_total_records_sync, board)

    def _get_total_records_sync(self, board: str) -> int:
        url = f"{self.base_url}/all"
        params = {"cat": "aa", "board": board}
        resp = requests.head(url, params=params, auth=self.auth)
        resp.raise_for_status()
        return int(resp.headers.get("x-total-count", 0))


class BoardsCache:
    _CACHE_EXPIRY = 3600

    def __init__(self, api: AALeaderboardAPI):
        self.api = api
        self.boards = []
        self.last_update = 0

    async def get_valid_boards(self) -> List[str]:
        await self._ensure_updated()
        return self._get_board_names()

    async def get_board_display_name(self, name: str) -> str | None:
        await self._ensure_updated()
        for board in self.boards:
            if board["name"] == name:
                return board["displayName"]
        return None

    def clear_cache(self):
        self.last_update = 0

    async def _ensure_updated(self):
        if time.time() - self.last_update > self._CACHE_EXPIRY:
            self.boards = await self.api.get_boards()
            print(f"Updated boards cache: {', '.join(self._get_board_names())}")
            self.last_update = time.time()

    def _get_board_names(self) -> List[str]:
        return [b["name"] for b in self.boards]


class QueryParser:
    def __init__(self, channel, args: List[str]):
        self.args = args
        self.channel = channel
        self.params = {}

    def parse(self) -> Dict:
        if not self.args:
            self.params["name"] = self.channel
            return self.params

        method_name = f"_parse_as_{self.args[0]}"
        if hasattr(self, method_name):
            getattr(self, method_name)()
        else:
            self._parse_general()
        return self.params

    def _parse_as_range(self):
        if len(self.args) < 3:
            raise ValueError("User provided range without start and end values")
        try:
            start = int(self.args[1])
            end = int(self.args[2])
        except ValueError:
            raise ValueError("User provided invalid range values")
        if start < 1 or end < start:
            raise ValueError("User provided invalid range values")
        self.params.update({"place": start, "take": end - start + 1})

    def _parse_as_top(self):
        if len(self.args) < 2:
            raise ValueError("User provided top without a value")
        try:
            n = int(self.args[1])
        except ValueError:
            raise ValueError("User provided invalid top value")
        self.params.update({"place": 1, "take": n})

    def _parse_general(self):
        arg = self.args[0]
        if ":" in arg:
            self._parse_time(arg)
        elif arg.isdigit():
            self.params.update({"place": int(arg)})
        else:
            self.params["name"] = " ".join(self.args)

    def _parse_time(self, arg: str):
        if arg[0] in ("<", ">"):
            operator = arg[0]
            time_str = arg[1:]
        else:
            operator = ">"
            time_str = arg
        time_val = self._parse_time_string(time_str)
        if operator == "<":
            self.params["ltetime"] = time_val
        else:
            self.params["gtetime"] = time_val

    @staticmethod
    def _parse_time_string(time_str: str) -> str:
        parts = list(map(int, time_str.split(":")))
        if len(parts) == 1:
            h, m, s = parts[0], 0, 0
        elif len(parts) == 2:
            h, m, s = parts[0], parts[1], 0
        else:
            h, m, s = parts
        return f"{h:02d}:{m:02d}:{s:02d}"


class ResponseFormatter:
    MAX_LENGTH = 250

    def __init__(self, results: List[Dict], board: str, multiple: bool = False):
        self.results = results
        self.board = board
        self.multiple = multiple

    def format(self) -> str:
        if not self.results:
            return "No results found."

        prefix = f"{self.board} "
        formatted = prefix + self._format_entries(show_time=True)
        if len(formatted) <= self.MAX_LENGTH:
            return formatted

        formatted = prefix + self._format_entries(show_time=False)
        return formatted if len(formatted) <= self.MAX_LENGTH else "Too many results."

    def _format_entries(self, show_time: bool) -> str:
        entries = []
        for i, result in enumerate(self.results):
            if not self.multiple and i != 0:
                break
            run = result["run"]
            entry = self._format_entry(run, show_time, show_place=i == 0)
            entries.append(entry)
        return " | ".join(entries)

    def _format_entry(self, run: Dict, show_time: bool, show_place: bool) -> str:
        place = f"#{run['place']}: " if show_place else ""
        players = self._format_players(run["players"])
        time = f" ({run['completionTime']})" if show_time else ""
        return f"{place}{players}{time}"

    @staticmethod
    def _format_players(players: List[str]) -> str:
        if not players:
            return "Unknown"
        if len(players) == 1:
            return players[0]
        return ", ".join(players[:-1]) + " & " + players[-1]


class AALeaderboard:
    def __init__(self, api: AALeaderboardAPI = None, cache: BoardsCache = None):
        self._api = api or AALeaderboardAPI(RoroConfig())
        self._boards = cache or BoardsCache(self._api)

    async def query(self, channel: str, args: List[str]) -> str | None:
        try:
            boards = await self._boards.get_valid_boards()
            board, query_args = self._parse_board(args, boards)
            board_name = await self._boards.get_board_display_name(board) or board

            if query_args and query_args[0] == "count":
                count = await self._api.get_total_records(board)
                return f"The {board_name} board has {count} records."

            params = QueryParser(channel, query_args).parse()
            results = await self._api.search(board, params)
            return ResponseFormatter(results, board_name, "take" in params).format()
        except Exception as e:
            logging.error(f"Error querying leaderboards: {e}")
            return None

    def _parse_board(self, args: List[str], valid_boards: List[str]) -> tuple:
        if args and args[0] in valid_boards:
            return args[0], args[1:]
        return "rsg", args
