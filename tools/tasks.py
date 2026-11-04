"""The 30 tasks for the AI player, in 3 levels.

Each task has:
  start     the files the OS boots with (#drawer / #page ... #end format)
  request   what the player is asked to do (plain words, no commands named)
  intent    a hand-written test of what the request asks (sanity check only)
  solution  a reference solution in key form (never shown to players)
  flex      optional {path: test} for answer pages whose wording may vary

Scoring (`strict_check`) is STRICT: the player's final files must equal the
expected end state (recorded from the reference solution in expected.json),
except `flex` pages, which only need to pass their test. validate_tasks.py
records expected.json and proves: the intent test agrees with it, the start
state fails, and planted mistakes (extra page, blanked page, copy left
behind) fail.

Files look like {"desk:letters": None, "desk:letters:mira": "text"}:
None means a drawer, a string is a page.
"""

import json
import re
from pathlib import Path

EXPECTED_FILE = Path(__file__).resolve().parent / "expected.json"


def norm(text):
    """Page text as a list of lines, trailing spaces and trailing blank lines removed."""
    if not isinstance(text, str):
        return None
    out = [l.rstrip() for l in text.split("\n")]
    while out and out[-1] == "":
        out.pop()
    return out


def is_drawer(f, p):
    return p in f and f[p] is None


def inside(f, drawer):
    """Paths directly inside a drawer."""
    return sorted(k for k in f if k.startswith(drawer + ":") and ":" not in k[len(drawer) + 1:])


def parse_start(text):
    """Files from start-files text, built the same way the OS builds its desk."""
    files, page, body = {}, None, []

    def drawers(path):
        parts = path.split(":")
        for i in range(1, len(parts) + 1):
            files.setdefault("desk:" + ":".join(parts[:i]), None)

    for line in text.splitlines():
        if page is not None:
            if line == "#end":
                if ":" in page:
                    drawers(page.rsplit(":", 1)[0])
                files["desk:" + page] = "\n".join(body)
                page, body = None, []
            else:
                body.append(line)
        elif line.startswith("#drawer "):
            drawers(line[8:].strip())
        elif line.startswith("#page "):
            page = line[6:].strip()
    return files


def canon(files):
    """Comparable form: drawers stay None, pages become normalized line lists."""
    return {k: (None if v is None else tuple(norm(v))) for k, v in files.items()}


_expected = None


def expected(task_id):
    global _expected
    if _expected is None:
        _expected = json.loads(EXPECTED_FILE.read_text()) if EXPECTED_FILE.exists() else {}
    return _expected.get(task_id)


def strict_check(task_id, files):
    """(passed, explanation): final files must equal the expected end state."""
    t = TASKS[task_id]
    want = expected(task_id)
    if want is None:
        return False, "no expected end state recorded (run validate_tasks.py)"
    start = parse_start(t["start"])
    problems = []
    for path in sorted(set(files) | set(want)):
        got, exp = files.get(path, "<missing>"), want.get(path, "<missing>")
        if path in t["flex"]:
            if not t["flex"][path](files.get(path)):
                problems.append(f"{path}: answer not accepted")
        elif path not in files:
            problems.append(f"missing {path}")
        elif path not in want:
            problems.append(f"unexpected {path}")
        elif (got is None) != (exp is None):
            problems.append(f"{path} should be a {'drawer' if exp is None else 'page'}")
        elif exp is not None and start.get(path) == exp:
            # A page the task leaves alone must stay EXACTLY the same.
            if got != exp:
                problems.append(f"changed {path}, which should stay exactly as it was")
        elif exp is not None and norm(got) != norm(exp):
            # A page the task creates or edits: trailing spaces / trailing blank lines don't matter.
            problems.append(f"wrong content in {path}")
    if not problems:
        return True, t["rule"]
    return False, f"{t['rule']} | " + "; ".join(problems[:5])


def task(level, request, start, intent, rule, solution, flex=None):
    return {"level": level, "request": request, "start": start.strip() + "\n", "intent": intent,
            "rule": rule, "solution": solution, "flex": flex or {}}


def first_line_has_code(code, others=()):
    """Flex test: the first line contains `code` as a whole number and none of `others`."""
    def test(text):
        if not isinstance(text, str):
            return False
        first = (norm(text) or [""])[0]
        return bool(re.search(rf"(?<!\d){code}(?!\d)", first)) and not any(o in first for o in others)
    return test


TASKS = {
    # ---------------------------------------------------------------- easy
    "copy-list": task(
        "easy",
        "Mira asked for the shopping list. Make a copy of the shopping list, put the copy in the "
        "letters drawer, and name it for-mira.",
        """
#page letters:mira
Hi! Could you send me the shopping list?
- Mira
#end
#page lists:shopping
eggs
bread
tea
#end""",
        lambda f: norm(f.get("desk:letters:for-mira")) == ["eggs", "bread", "tea"]
        and norm(f.get("desk:lists:shopping")) == ["eggs", "bread", "tea"],
        "letters:for-mira holds eggs/bread/tea and the original stays",
        "copy lists:shopping letters:for-mira{enter}",
    ),
    "read-code": task(
        "easy",
        "The page called code in the vault drawer contains a door code. Make a page called answer "
        "on the desk that contains just that code.",
        """
#page vault:code
The door code is 4817.
#end
#page welcome
Hello and welcome.
#end""",
        lambda f: norm(f.get("desk:answer")) == ["4817"],
        "desk:answer contains just 4817",
        "write answer 4817{enter}",
    ),
    "new-drawer": task(
        "easy",
        "Make a new, empty drawer on the desk called photos.",
        """
#drawer music
#page welcome
Hello and welcome.
#end""",
        lambda f: is_drawer(f, "desk:photos") and not inside(f, "desk:photos"),
        "an empty drawer desk:photos exists",
        "build photos{enter}",
    ),
    "rename-page": task(
        "easy",
        "Rename the page draft in the notes drawer to final.",
        """
#page notes:draft
My first story.
#end
#page notes:ideas
A robot cat.
#end""",
        lambda f: f.get("desk:notes:final") == "My first story." and "desk:notes:draft" not in f
        and f.get("desk:notes:ideas") == "A robot cat.",
        "notes:final has the draft's text, notes:draft is gone, notes:ideas untouched",
        "rename notes:draft final{enter}",
    ),
    "toss-page": task(
        "easy",
        "Throw away the page old-todo in the work drawer. Keep everything else.",
        """
#page work:old-todo
Buy stamps.
#end
#page work:todo
Call the bank.
#end
#page work:report
Sales went up.
#end""",
        lambda f: "desk:work:old-todo" not in f and f.get("desk:work:todo") == "Call the bank."
        and f.get("desk:work:report") == "Sales went up.",
        "work:old-todo is gone, work:todo and work:report stay",
        "toss work:old-todo{enter}",
    ),
    "add-line": task(
        "easy",
        "Add milk as a new last line of the shopping list (the page shopping in the lists drawer).",
        """
#page lists:shopping
eggs
bread
tea
#end""",
        lambda f: norm(f.get("desk:lists:shopping")) == ["eggs", "bread", "tea", "milk"],
        "lists:shopping is eggs/bread/tea/milk",
        "add lists:shopping milk{enter}",
    ),
    "move-page": task(
        "easy",
        "Move the page invoice from the desk into the bills drawer.",
        """
#page invoice
Phone bill: 30 euro.
#end
#page bills:march
Water bill: 12 euro.
#end""",
        lambda f: f.get("desk:bills:invoice") == "Phone bill: 30 euro." and "desk:invoice" not in f
        and f.get("desk:bills:march") == "Water bill: 12 euro.",
        "bills:invoice exists with its text, desk:invoice is gone",
        "move invoice bills{enter}",
    ),
    "write-note": task(
        "easy",
        "Make a page called hello on the desk with exactly this text: Hello, Mira!",
        """
#page welcome
Hello and welcome.
#end""",
        lambda f: norm(f.get("desk:hello")) == ["Hello, Mira!"],
        "desk:hello is exactly 'Hello, Mira!'",
        "write hello Hello, Mira!{enter}",
    ),
    "find-word": task(
        "easy",
        "One page somewhere mentions a pumpkin. Rename that page to pumpkin-plan, keeping it in "
        "the same drawer.",
        """
#page notes:week
Clean the car.
#end
#page garden:plan
Plant a pumpkin in May.
#end
#page garden:tools
spade, rake
#end
#page kitchen:menu
tomato soup
#end""",
        lambda f: f.get("desk:garden:pumpkin-plan") == "Plant a pumpkin in May."
        and "desk:garden:plan" not in f and f.get("desk:garden:tools") == "spade, rake",
        "garden:plan renamed to garden:pumpkin-plan, others untouched",
        "rename garden:plan pumpkin-plan{enter}",
    ),
    "toss-empty": task(
        "easy",
        "Throw away the empty drawer called trash-me.",
        """
#drawer trash-me
#page keep-me:note
Do not lose this.
#end""",
        lambda f: "desk:trash-me" not in f and f.get("desk:keep-me:note") == "Do not lose this.",
        "desk:trash-me is gone, keep-me:note stays",
        "toss trash-me{enter}",
    ),
    # -------------------------------------------------------------- medium
    "change-word": task(
        "medium",
        "In the page reply in the letters drawer, change the word Tuesday to Friday. Do not change "
        "anything else.",
        """
#page letters:reply
Dear Mira,
See you on Tuesday at noon.
Love, Sam
#end""",
        lambda f: norm(f.get("desk:letters:reply")) == ["Dear Mira,", "See you on Friday at noon.", "Love, Sam"],
        "letters:reply is the same 3 lines with Friday instead of Tuesday",
        "notes letters:reply{enter}{down}{right*11}{del*7}Friday{ctrl+k}{esc}",
    ),
    "packing": task(
        "medium",
        "Make a new page called packing in the lists drawer with three lines: tent, rope and lamp "
        "(one item per line, in that order).",
        """
#page lists:shopping
eggs
bread
#end""",
        lambda f: norm(f.get("desk:lists:packing")) == ["tent", "rope", "lamp"],
        "lists:packing is tent/rope/lamp",
        "write lists:packing tent{enter}add lists:packing rope{enter}add lists:packing lamp{enter}",
    ),
    "album": task(
        "medium",
        "Make a new drawer called album and move every page whose name starts with photo- into it. "
        "Leave all other pages where they are.",
        """
#page photo-beach
Sunny day at the sea.
#end
#page photo-snow
The mountains in winter.
#end
#page photo-cat
Our cat asleep.
#end
#page photography-tips
Use the morning light.
#end""",
        lambda f: inside(f, "desk:album") == ["desk:album:photo-beach", "desk:album:photo-cat", "desk:album:photo-snow"]
        and f.get("desk:album:photo-beach") == "Sunny day at the sea." and f.get("desk:album:photo-cat") == "Our cat asleep."
        and f.get("desk:album:photo-snow") == "The mountains in winter."
        and not any(k.startswith("desk:photo-") for k in f)
        and f.get("desk:photography-tips") == "Use the morning light.",
        "album holds exactly the 3 photo- pages; photography-tips stays on the desk",
        "build album{enter}move photo-beach album{enter}move photo-snow album{enter}move photo-cat album{enter}",
    ),
    "nested": task(
        "medium",
        "Make drawers so that the path projects:2026:ideas exists (ideas inside 2026 inside "
        "projects), then make a page called first inside ideas with the text robot cat.",
        """
#page welcome
Hello and welcome.
#end""",
        lambda f: is_drawer(f, "desk:projects:2026:ideas")
        and norm(f.get("desk:projects:2026:ideas:first")) == ["robot cat"],
        "drawer projects:2026:ideas exists with page first = 'robot cat'",
        "build projects{enter}build projects:2026{enter}build projects:2026:ideas{enter}"
        "write projects:2026:ideas:first robot cat{enter}",
    ),
    "swap": task(
        "medium",
        "The pages monday and tuesday got mixed up: monday holds Tuesday's plan and tuesday holds "
        "Monday's plan. Swap their names so each page ends up with the other one's name. "
        "Don't leave any extra pages behind.",
        """
#page monday
Piano lesson
#end
#page tuesday
Gym
#end""",
        lambda f: f.get("desk:monday") == "Gym" and f.get("desk:tuesday") == "Piano lesson"
        and sorted(f) == ["desk:monday", "desk:tuesday"],
        "monday = Gym, tuesday = Piano lesson, nothing else on the desk",
        "rename monday temp{enter}rename tuesday monday{enter}rename temp tuesday{enter}",
    ),
    "merge": task(
        "medium",
        "Make a page called full on the desk that has all lines of part1 followed by all lines of "
        "part2. Keep part1 and part2 as they are.",
        """
#page part1
Once upon a time
there was a cat.
#end
#page part2
The cat liked tea.
The end.
#end""",
        lambda f: norm(f.get("desk:full")) == ["Once upon a time", "there was a cat.", "The cat liked tea.", "The end."]
        and norm(f.get("desk:part1")) == ["Once upon a time", "there was a cat."]
        and norm(f.get("desk:part2")) == ["The cat liked tea.", "The end."],
        "full = part1 lines then part2 lines; part1 and part2 unchanged",
        "copy part1 full{enter}add full The cat liked tea.{enter}add full The end.{enter}",
    ),
    "count": task(
        "medium",
        "Count all pages inside the archive drawer, including pages in drawers inside it. Make a "
        "page called count on the desk containing just that number.",
        """
#page archive:a
one
#end
#page archive:b
two
#end
#page archive:old:c
three
#end
#page archive:old:d
four
#end
#page archive:old:e
five
#end
#page archive:old:older:f
six
#end
#page notes:g
not in the archive
#end""",
        lambda f: norm(f.get("desk:count")) == ["6"],
        "desk:count contains just 6",
        "write count 6{enter}",
    ),
    "clean-junk": task(
        "medium",
        "Throw away every page in the junk drawer, and then the junk drawer itself. Don't touch "
        "anything else.",
        """
#page junk:old-ad
50% off shoes!
#end
#page junk:spam
You won a prize!
#end
#page junk:receipt-2019
Coffee 2 euro
#end
#page mail:letter
Dear Sam, how are you?
#end""",
        lambda f: not any(k == "desk:junk" or k.startswith("desk:junk:") for k in f)
        and f.get("desk:mail:letter") == "Dear Sam, how are you?",
        "no junk drawer or junk pages left; mail:letter stays",
        "toss junk:old-ad{enter}toss junk:spam{enter}toss junk:receipt-2019{enter}toss junk{enter}",
    ),
    "fix-typo": task(
        "medium",
        "The page soup in the recipes drawer has a spelling mistake: onoin. Change it to onion. "
        "Do not change anything else.",
        """
#page recipes:soup
Onion soup
Cut one onoin into rings.
Cook for 20 minutes.
#end""",
        lambda f: norm(f.get("desk:recipes:soup")) == ["Onion soup", "Cut one onion into rings.", "Cook for 20 minutes."],
        "recipes:soup is the same 3 lines with onion spelled right",
        "notes recipes:soup{enter}{down}{right*8}{del*5}onion{ctrl+k}{esc}",
    ),
    "restore": task(
        "medium",
        "The page meeting-notes was put in the trash drawer by mistake. Move it back into the work "
        "drawer, then throw away the trash drawer (it will be empty).",
        """
#page trash:meeting-notes
Plan the summer party.
#end
#page work:schedule
Monday 9:00 stand-up
#end""",
        lambda f: f.get("desk:work:meeting-notes") == "Plan the summer party." and "desk:trash" not in f
        and f.get("desk:work:schedule") == "Monday 9:00 stand-up",
        "work:meeting-notes exists, trash drawer is gone",
        "move trash:meeting-notes work{enter}toss trash{enter}",
    ),
    # ---------------------------------------------------------------- hard
    "locker": task(
        "hard",
        "Mira wrote you a letter (in the letters drawer). Answer her question: make a page called "
        "reply-mira in the letters drawer whose first line is the answer.",
        """
#page letters:mira
Hi! What is the code for my gym locker? You wrote it down somewhere.
- Mira
#end
#page misc:numbers
bike lock: 5520
gym locker: 2391
wifi: 7788
#end
#page misc:birthdays
Ana: 3 May
#end""",
        lambda f: first_line_has_code("2391", ("5520", "7788"))(f.get("desk:letters:reply-mira")),
        "letters:reply-mira's first line contains the code 2391 (and not the other codes)",
        "write letters:reply-mira 2391{enter}",
        flex={"desk:letters:reply-mira": first_line_has_code("2391", ("5520", "7788"))},
    ),
    "sort": task(
        "hard",
        "Sort the lines of the page names in the lists drawer alphabetically (A to Z). Keep the "
        "result in the same page.",
        """
#page lists:names
Zoe
Adam
Mira
Ben
Leo
#end""",
        lambda f: norm(f.get("desk:lists:names")) == ["Adam", "Ben", "Leo", "Mira", "Zoe"],
        "lists:names is Adam/Ben/Leo/Mira/Zoe",
        "write lists:names Adam{enter}add lists:names Ben{enter}add lists:names Leo{enter}"
        "add lists:names Mira{enter}add lists:names Zoe{enter}",
    ),
    "dedupe": task(
        "hard",
        "The page guests in the lists drawer has some names more than once. Remove the repeats so "
        "each name appears once, keeping the order in which the names first appear.",
        """
#page lists:guests
Ana
Tom
Ana
Lina
Tom
Omar
#end""",
        lambda f: norm(f.get("desk:lists:guests")) == ["Ana", "Tom", "Lina", "Omar"],
        "lists:guests is Ana/Tom/Lina/Omar",
        "write lists:guests Ana{enter}add lists:guests Tom{enter}add lists:guests Lina{enter}"
        "add lists:guests Omar{enter}",
    ),
    "budget": task(
        "hard",
        "The page spent in the money drawer lists amounts, one per line. Make a page called total "
        "in the money drawer containing just the sum of all the amounts.",
        """
#page money:spent
food 12
bus 3
cinema 9
books 26
#end""",
        lambda f: norm(f.get("desk:money:total")) == ["50"],
        "money:total contains just 50",
        "write money:total 50{enter}",
    ),
    "urgent": task(
        "hard",
        "Some pages contain the word urgent (in any capitals). Make a new drawer called today on "
        "the desk and move every such page into it. Leave the other pages where they are.",
        """
#page work:email
This is urgent, reply today.
#end
#page work:notes
Nothing special.
#end
#page home:bills
Pay the rent - URGENT
#end
#page home:garden
Water the plants.
#end""",
        lambda f: inside(f, "desk:today") == ["desk:today:bills", "desk:today:email"]
        and f.get("desk:today:email") == "This is urgent, reply today." and f.get("desk:today:bills") == "Pay the rent - URGENT"
        and "desk:work:email" not in f and "desk:home:bills" not in f
        and f.get("desk:work:notes") == "Nothing special." and f.get("desk:home:garden") == "Water the plants.",
        "today holds exactly email and bills; work:notes and home:garden stay",
        "build today{enter}move work:email today{enter}move home:bills today{enter}",
    ),
    "rename-deep": task(
        "hard",
        "Rename the drawer old-name to clients. Everything inside it must stay inside it.",
        """
#page old-name:acme:contact
Jo Smith, 555-0101
#end
#page old-name:list
acme
#end""",
        lambda f: f.get("desk:clients:acme:contact") == "Jo Smith, 555-0101" and f.get("desk:clients:list") == "acme"
        and not any(k.startswith("desk:old-name") for k in f),
        "clients:acme:contact and clients:list exist; old-name is gone",
        "rename old-name clients{enter}",
    ),
    "invites": task(
        "hard",
        "For each person listed in the page list in the people drawer, make a copy of the page "
        "letter from the templates drawer. Put the copies in a new drawer called outbox on the "
        "desk, each named letter- followed by the person's name (for example letter-ana).",
        """
#page templates:letter
Dear friend,
You are invited to my party!
#end
#page people:list
ana
omar
lina
#end""",
        lambda f: inside(f, "desk:outbox") == ["desk:outbox:letter-ana", "desk:outbox:letter-lina", "desk:outbox:letter-omar"]
        and all(norm(f[k]) == ["Dear friend,", "You are invited to my party!"] for k in inside(f, "desk:outbox"))
        and norm(f.get("desk:templates:letter")) == ["Dear friend,", "You are invited to my party!"],
        "outbox holds letter-ana/omar/lina with the template text; template stays",
        "build outbox{enter}copy templates:letter outbox:letter-ana{enter}"
        "copy templates:letter outbox:letter-omar{enter}copy templates:letter outbox:letter-lina{enter}",
    ),
    "instructions": task(
        "hard",
        "Follow the instructions in the page start-here.",
        """
#page start-here
1. Build a drawer called done.
2. Move the page task-a into done.
3. In done, make a page called status with the text finished.
#end
#page task-a
Paint the fence.
#end
#page task-b
Fix the bike.
#end""",
        lambda f: f.get("desk:done:task-a") == "Paint the fence." and norm(f.get("desk:done:status")) == ["finished"]
        and "desk:task-a" not in f and f.get("desk:task-b") == "Fix the bike.",
        "done holds task-a and status=finished; task-b stays on the desk",
        "build done{enter}move task-a done{enter}write done:status finished{enter}",
    ),
    "insert-line": task(
        "hard",
        "In the page week in the plan drawer, add a new line Wednesday: gym between the Tuesday "
        "and Thursday lines. Do not change anything else.",
        """
#page plan:week
Monday: swim
Tuesday: piano
Thursday: chess
Friday: rest
#end""",
        lambda f: norm(f.get("desk:plan:week")) == ["Monday: swim", "Tuesday: piano", "Wednesday: gym", "Thursday: chess", "Friday: rest"],
        "plan:week has Wednesday: gym as the new 3rd line, rest unchanged",
        "notes plan:week{enter}{down}{end}{enter}Wednesday: gym{ctrl+k}{esc}",
    ),
    "triage": task(
        "hard",
        "Sort the inbox drawer: make drawers called invoices and notes on the desk, move every page "
        "whose name starts with invoice- into invoices and every page whose name starts with note- "
        "into notes. Then throw away the inbox drawer, which will be empty.",
        """
#page inbox:invoice-may
Invoice: 40 euro
#end
#page inbox:invoice-june
Invoice: 35 euro
#end
#page inbox:note-dentist
Dentist on Monday.
#end
#page inbox:note-call-mom
Call mom on Sunday.
#end""",
        lambda f: inside(f, "desk:invoices") == ["desk:invoices:invoice-june", "desk:invoices:invoice-may"]
        and inside(f, "desk:notes") == ["desk:notes:note-call-mom", "desk:notes:note-dentist"]
        and f.get("desk:invoices:invoice-may") == "Invoice: 40 euro" and f.get("desk:invoices:invoice-june") == "Invoice: 35 euro"
        and f.get("desk:notes:note-dentist") == "Dentist on Monday." and f.get("desk:notes:note-call-mom") == "Call mom on Sunday."
        and not any(k == "desk:inbox" or k.startswith("desk:inbox:") for k in f),
        "invoices and notes hold the right 2 pages each; inbox is gone",
        "build invoices{enter}build notes{enter}move inbox:invoice-may invoices{enter}"
        "move inbox:invoice-june invoices{enter}move inbox:note-dentist notes{enter}"
        "move inbox:note-call-mom notes{enter}toss inbox{enter}",
    ),
}
