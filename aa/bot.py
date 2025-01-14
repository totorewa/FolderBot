from typing import Callable
from twitchio.ext import commands
from daemon import seconds_since_update, duration_since_update
from query import DATA, PacemanObject, DATA_SORTED, ALL_SPLITS, USEFUL_DATA, td


def partition(l: list, p: Callable):
    has_it = [x for x in l if p(x)]
    nopers = [x for x in l if not p(x)]
    return (has_it, nopers)


def pctg(a, b):
    return f'{round(100 * b / a, 2)}'

class Bot(commands.Bot):

    def __init__(self):
        # Initialise our Bot with our access token, prefix and a list of channels to join on boot...
        # prefix can be a callable, which returns a list of strings or a string...
        # initial_channels can also be a callable which returns a list of strings...
        super().__init__(token=open('auth/ttg-access.txt').read().strip(), prefix='?', initial_channels=['folderbot', 'desktopfolder'])

    async def event_ready(self):
        # Notify us when everything is ready!
        # We are logged in and ready to chat and use commands...
        print(f'Logged in as | {self.nick}')
        print(f'User id is | {self.user_id}')

    @commands.command()
    async def hello(self, ctx: commands.Context):
        # Here we have a command hello, we can invoke our command with our prefix and command name
        # e.g ?hello
        # We can also give our commands aliases (different names) to invoke with.

        # Send a hello back!
        # Sending a reply back to the channel is easy... Below is an example.
        await ctx.send(f'Hello {ctx.author.name}!')

    @commands.command()
    async def help(self, ctx: commands.Context):
        await ctx.send("AA Paceman extension: ?average [splitname] [playername] -> average split for a player, ?info -> Metadata on bot")

    @commands.command()
    async def info(self, ctx: commands.Context):
        dur0 = duration_since_update()
        data = DATA_SORTED()
        dur = duration_since_update()
        infos = [f'Time since update: {dur}.']
        if dur0 != dur:
            infos.append(f'({dur0} before this command)')
        infos.append(f'{len(data)} known AA runs.')
        last_nether = PacemanObject(data[0])
        if last_nether.nether is not None:
            infos.append(f'Last nether: {last_nether.nether_str()} by {last_nether.player}.')
        await ctx.send(' '.join(infos))

    @commands.command()
    async def average(self, ctx: commands.Context, splitname: str, playername: str = "DesktopFolder"):
        splitname = splitname.lower()
        if not splitname in ALL_SPLITS:
            return await ctx.send(f'{splitname} is not a valid AA split: {ALL_SPLITS}')
        pcs = [p for p in USEFUL_DATA() if p.filter(split=splitname, player=playername)]
        if len(pcs) == 0:
            return await ctx.send(f'{playername} has no known {splitname} AA splits.')
        await ctx.send(f'Average AA {splitname} for {playername}: {td.average(ts=[pc.get(splitname) for pc in pcs])} (sample: {len(pcs)})')

    @commands.command()
    async def conversion(self, ctx: commands.Context, split1: str, split2: str, playername: str = "DesktopFolder"):
        # yikes need to do some refactoring
        split1 = split1.lower()
        split2 = split2.lower()
        for split in [split1, split2]:
            if not split in ALL_SPLITS:
                return await ctx.send(f'{split} is not a valid AA split: {ALL_SPLITS}')

        pcs = [p for p in USEFUL_DATA() if p.filter(split=split1, player=playername)]
        if len(pcs) == 0:
            return await ctx.send(f'{playername} has no known {split1} AA splits.')
        n = len(pcs)
        x = len([p for p in pcs if p.has(split2)])
        await ctx.send(f'{pctg(n, x)}% of {playername}\'s AA {split1} splits lead to starting {split2} splits.')

    @commands.command()
    async def bastion_breakdown(self, ctx: commands.Context, playername: str = "DesktopFolder"):
        pcs = [p for p in USEFUL_DATA() if p.filter(split='nether', player=playername)]
        if len(pcs) == 0:
            return await ctx.send(f'{playername} has no known AA nethers.')

        def pctgwith(l: list[PacemanObject]):
            n = len(l)
            x = len([p for p in l if p.has('bastion')])
            return pctg(n, x)

        def writer(l: list[PacemanObject], s: str):
            if not l:
                return ''
            return f'Conversion for {s} nethers: {pctgwith(l)}% ({len(l)})'

        sub_330, pcs = partition(pcs, lambda p: p.nether.total_seconds() < 60 * 3 + 30)
        sub_400, pcs = partition(pcs, lambda p: p.nether.total_seconds() < 60 * 4)
        sub_430, pcs = partition(pcs, lambda p: p.nether.total_seconds() < 60 * 3 + 30)
        sub_500, pcs = partition(pcs, lambda p: p.nether.total_seconds() < 60 * 5)
        brk = ' | '.join([x for x in [
                writer(sub_330, '< 3:30'),
                writer(sub_400, '3:30-4:00'),
                writer(sub_430, '4:00-4:30'),
                writer(sub_500, '4:30-5:00'),
                writer(pcs, '> 5:00'),
            ]
            if x != '' 
        ])
        await ctx.send(f'Bastion conversion breakdown for {playername}: {brk}')



bot = Bot()
bot.run()
