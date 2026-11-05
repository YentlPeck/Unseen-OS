"""Version 3 tasks: a big world where chores (small programs) are the practical way.

Four kinds (docs/v3-design.md):
  count  add up or cross-check, then save just the answer on desk:answer
  bulk   many sends or events at once
  fix    a chore in desk:chores has a bug: fix it and run it once
  write  write a reusable chore at a named page and run it once

Every world has the drawer desk:chores. Players may keep any pages they like
there: the scorer ignores desk:chores, except the task's own chore page for
`fix` and `write` tasks, which is checked by what it DOES: it is re-run
unchanged on 3 hidden worlds (one of them with nothing to find) and the
results are checked by an independent checker.

Each task has a reference chore. Its key solution types the chore in Notes
and runs it, exactly as a player could (validate_tasks.py records the end
state from it and checks it against the task's own checker).
"""

import copy

import world_v3 as W
from tasks import task

SEED = 20261112


def visible_world():
    w = W.generate(SEED)
    # a few invoices from senders who are not contacts (a trap for "from contacts")
    w["inbox"][7] = {"from": "shop", "subject": "Invoice", "body": "Total: 240 euro. Please pay before 2026-11-30."}
    w["inbox"][61] = {"from": "bank", "subject": "Invoice", "body": "Total: 118.5 euro. Please pay before 2026-12-04."}
    # three birthdays today
    for i in (5, 40, 77):
        w["contacts"][i]["birthday"] = W.TODAY[5:]
    return w


VISIBLE = visible_world()


def chore_keys(page, source, replace=False):
    """Key solution: type the chore in Notes (line by line), keep it, run it, close the output."""
    keys = f"toss {page}{{enter}}" if replace else ""
    keys += f"notes {page}{{enter}}" + "{enter}".join(source.strip("\n").split("\n"))
    return keys + f"{{ctrl+k}}{{esc}}do {page}{{enter}}{{esc}}"


def one_number(value):
    """Flex test for desk:answer: exactly one number in the text, equal to value."""
    import re

    def test(text):
        if not isinstance(text, str):
            return False
        nums = re.findall(r"(?<![\w.])-?\d+(?:\.\d+)?(?![\w.]*\d)", text)
        return len(nums) == 1 and float(nums[0]) == value
    return test


def answer_task(level, request, ref, oracle, wrong_values=(), world=None):
    """A `count` task: the answer page desk:answer is checked by its number."""
    world = world or VISIBLE
    value = oracle(W.render(world))
    return task(level, request, W.render(world), lambda f: one_number(value)(f.get("desk:answer")),
                f"desk:answer holds {value:g}",
                chore_keys("desk:chores:job", ref),
                flex={"desk:answer": one_number(value)},
                right=[("desk:answer", f"{value:g}"), ("desk:answer", f"{value:g}\n")],
                wrong=[("desk:answer", f"{v:g}") for v in wrong_values] + [("desk:answer", "")]) | {
        "v3": "count", "oracle_value": value, "ref_chore": ref}


def files_of(start_text):
    from tasks import parse_start
    return parse_start(start_text)


# =========================================================================== count

REF_BROOK = """set total to 0
for each m in pages('mail:inbox')
  set who to 'contacts:' & field(m, 'from')
  when field(m, 'subject') is 'Invoice' and exists(who)
    when field(who, 'city') is 'Brookville'
      set total to total + number(item(words(body(m)), 2))
    done
  done
done
write('desk:answer', total)"""


def oracle_brook(start):
    f = files_of(start)
    c = W.contacts(f)
    return sum(W.invoice_amount(m)[1] for m in W.inbox(f)
               if m["subject"] == "Invoice" and m["from"] in c and c[m["from"]]["city"] == "Brookville")


def oracle_strangers(start):
    f = files_of(start)
    c = W.contacts(f)
    return sum(1 for e in W.events(f) if e["date"].startswith("2026-12") and e["with"] and e["with"] not in c)


REF_STRANGERS = """set n to 0
for each e in pages('calendar')
  set w to field(e, 'with')
  when field(e, 'date') >= '2026-12-01' and w is not ''
    when not exists('contacts:' & w)
      set n to n + 1
    done
  done
done
write('desk:answer', n)"""


def oracle_weekend(start):
    f = files_of(start)
    return sum(1 for e in W.events(f) if W.weekday(e["date"]) in ("saturday", "sunday"))


REF_WEEKEND = """set n to 0
for each e in pages('calendar')
  set day to weekday(field(e, 'date'))
  when day is 'saturday' or day is 'sunday'
    set n to n + 1
  done
done
write('desk:answer', n)"""


def oracle_never(start):
    f = files_of(start)
    written = {s["to"] for s in W.sent(f)}
    return sum(1 for m in W.inbox(f) if m["from"] not in written)


REF_NEVER = """set written to ''
for each s in pages('mail:sent')
  set written to written & ' ' & field(s, 'to')
done
set n to 0
for each m in pages('mail:inbox')
  when not (words(written) has field(m, 'from'))
    set n to n + 1
  done
done
write('desk:answer', n)"""


def oracle_biggest(start):
    f = files_of(start)
    c = W.contacts(f)
    best = max((m for m in W.inbox(f) if m["subject"] == "Invoice" and m["from"] in c),
               key=lambda m: W.invoice_amount(m)[1])
    return W.invoice_amount(best)[1]


REF_BIGGEST = """set best to 0
for each m in pages('mail:inbox')
  set who to 'contacts:' & field(m, 'from')
  when field(m, 'subject') is 'Invoice' and exists(who)
    set amount to number(item(words(body(m)), 2))
    when amount > best
      set best to amount
    done
  done
done
write('desk:answer', best)"""

SCRATCH = ("You may write chores (and any other pages) in the drawer desk:chores; "
           "everything in it is ignored.")

COUNT = {
    "brook-invoices": answer_task(
        "count",
        "Some mails in your inbox are invoices (their subject is Invoice). Add up the amounts of "
        "the invoices sent by contacts who live in Brookville, and save just the total on the "
        "page desk:answer. " + SCRATCH,
        REF_BROOK, oracle_brook, wrong_values=(oracle_brook(W.render(VISIBLE)) + 1,)),
    "stranger-events": answer_task(
        "count",
        "How many events in December 2026 are with someone who is not one of your contacts? "
        "(Events where With is empty don't count.) Save just the number on the page desk:answer. "
        + SCRATCH,
        REF_STRANGERS, oracle_strangers),
    "weekend-events": answer_task(
        "count",
        "How many of your events are on a Saturday or a Sunday? Save just the number on the page "
        "desk:answer. " + SCRATCH,
        REF_WEEKEND, oracle_weekend),
    "never-written": answer_task(
        "count",
        "How many mails in your inbox are from someone you have never sent a mail to (nothing in "
        "Sent is addressed to them)? Save just the number on the page desk:answer. " + SCRATCH,
        REF_NEVER, oracle_never),
    "biggest-invoice": answer_task(
        "count",
        "Of the invoices in your inbox (subject Invoice) that come from your contacts, which has "
        "the highest amount? Save just that amount on the page desk:answer. " + SCRATCH,
        REF_BIGGEST, oracle_biggest),
}

# ============================================================================ bulk


def bulk_task(request, ref, expect_sent=None, expect_events=None, world=None):
    """A `bulk` task: exactly the expected new mails / events must exist (checked by content)."""
    start = W.render(world or VISIBLE)

    def extra(files):
        from tasks import parse_start
        s = parse_start(start)
        ok = True
        if expect_sent is not None:
            ok &= W.new_sent(s, files) == expect_sent(s)
        if expect_events is not None:
            ok &= W.new_events(s, files) == expect_events(s)
        return ok
    return task("bulk", request, start, extra, "exactly the right mails/events were added",
                chore_keys("desk:chores:job", ref), extra=extra) | {"v3": "bulk", "ref_chore": ref}


INVITE_TEXT = "Street party on Saturday at 15:00. Hope you can come!"


def expect_invite(s):
    return sorted((m, "Street party", INVITE_TEXT) for m, c in W.contacts(s).items() if "neighbour" in c["note"])


REF_INVITE = f"""set text to '{INVITE_TEXT}'
for each c in pages('contacts')
  when field(c, 'note') has 'neighbour'
    send(field(c, 'mail'), 'Street party', text)
  done
done"""


def expect_thanks(s):
    c = W.contacts(s)
    out = []
    for m in W.inbox(s):
        if m["subject"] == "Invoice" and m["from"] in c:
            text, value = W.invoice_amount(m)
            if value > 100:
                out.append((m["from"], "Invoice received", f"Thanks, I will pay {text} euro."))
    return sorted(out)


REF_THANKS = """for each m in pages('mail:inbox')
  set who to field(m, 'from')
  when field(m, 'subject') is 'Invoice' and exists('contacts:' & who)
    set amount to item(words(body(m)), 2)
    when number(amount) > 100
      send(who, 'Invoice received', 'Thanks, I will pay ' & amount & ' euro.')
    done
  done
done"""


def expect_planning(s):
    evs = W.events(s)
    out = []
    for i in range(31):
        d = W.date_add("2026-12-01", i)
        if W.weekday(d) in ("saturday", "sunday"):
            continue
        if not any(e["date"] == d and e["time"] < "12:00" for e in evs):
            out.append((d, "08:30", "Planning", ""))
    return out


REF_PLANNING = """set d to '2026-12-01'
repeat 31 times
  set day to weekday(d)
  when day is not 'saturday' and day is not 'sunday'
    set busy to no
    for each e in pages('calendar')
      when field(e, 'date') is d and field(e, 'time') < '12:00'
        set busy to yes
      done
    done
    when not busy
      event(d, '08:30', 'Planning', '')
    done
  done
  set d to add_days(d, 1)
done"""


def expect_reminders(s):
    c = W.contacts(s)
    return sorted((e["with"], "Reminder", f"See you at {e['title']} on {e['date']} at {e['time']}!")
                  for e in W.events(s) if "2026-11-12" <= e["date"] <= "2026-12-02" and e["with"] in c)


REF_REMINDERS = """for each e in pages('calendar')
  set d to field(e, 'date')
  set w to field(e, 'with')
  when d >= '2026-11-12' and d <= '2026-12-02' and w is not ''
    when exists('contacts:' & w)
      set t to 'See you at ' & field(e, 'title') & ' on ' & d
      send(w, 'Reminder', t & ' at ' & field(e, 'time') & '!')
    done
  done
done"""

REF_PHONEBOOK = """write('desk:phonebook', '')
set first to yes
for each c in pages('contacts')
  when field(c, 'city') is 'Brookville'
    set line to field(c, 'name') & ': ' & field(c, 'phone')
    when first
      write('desk:phonebook', line)
      set first to no
    otherwise
      add('desk:phonebook', line)
    done
  done
done"""


def phonebook_text(s):
    return "\n".join(f"{c['name']}: {c['phone']}" for m, c in W.contacts(s).items() if c["city"] == "Brookville")


BULK = {
    "invite-neighbours": bulk_task(
        "Send every contact whose note mentions neighbour a mail with the subject Street party and "
        f"the text: {INVITE_TEXT} " + SCRATCH,
        REF_INVITE, expect_sent=expect_invite),
    "invoice-thanks": bulk_task(
        "For every invoice in your inbox (subject Invoice) from one of your contacts with an amount "
        "of more than 100 euro, send the sender a mail with the subject Invoice received and the "
        "text: Thanks, I will pay N euro.  (N is the amount exactly as the invoice writes it.) "
        + SCRATCH,
        REF_THANKS, expect_sent=expect_thanks),
    "december-planning": bulk_task(
        "Add an event with the title Planning at 08:30 (With empty) on every weekday (Monday to "
        "Friday) in December 2026 that has no event before 12:00 yet. " + SCRATCH,
        REF_PLANNING, expect_events=expect_planning),
    "reminders": bulk_task(
        "For every event from today (2026-11-12) up to and including 2026-12-02 that is with one of "
        "your contacts, send that contact a mail with the subject Reminder and the text: See you "
        "at TITLE on DATE at TIME!  (with the event's title, date and time, e.g. See you at Lunch "
        "on 2026-11-20 at 12:30!) " + SCRATCH,
        REF_REMINDERS, expect_sent=expect_reminders),
    "phonebook": task(
        "bulk",
        "Make a page desk:phonebook with one line for every contact who lives in Brookville, "
        "written Full Name: phone (e.g. Ana Costa: 555-1234), in alphabetical order of their mail "
        "addresses. " + SCRATCH,
        W.render(VISIBLE), lambda f: f.get("desk:phonebook") == phonebook_text(files_of(W.render(VISIBLE))),
        "desk:phonebook lists the Brookville contacts",
        chore_keys("desk:chores:job", REF_PHONEBOOK)) | {"v3": "bulk", "ref_chore": REF_PHONEBOOK},
}

# ===================================================================== fix and write


def hidden_worlds(mutate_empty, seeds=(SEED + 101, SEED + 202, SEED + 303)):
    """Three hidden worlds: a fresh random one, one with nothing to find, one smaller with
    another date. Each is a function of the chore source (the chore page is added)."""
    a = W.generate(seeds[0])
    b = W.generate(seeds[1])
    mutate_empty(b)
    c = W.generate(seeds[2], n_contacts=40, n_mails=60, n_sent=8, n_events=30, today="2026-12-03")
    for i in (3, 17):  # birthdays on the smaller world's date
        c["contacts"][i]["birthday"] = c["today"][5:]
    return [a, b, c]


def with_chore(world, page, source):
    w = copy.deepcopy(world)
    w["pages"][page[len("desk:"):]] = source
    return W.render(w)


def code_task(kind, request, page, start_world, ref, check, empty, wrong_chores=(), level=None,
              outputs=(), adds_mail=False):
    """A `fix` or `write` task. check(start_files, end_files, today) -> (ok, why) is the
    independent checker, used on the visible world and on every hidden world."""
    start = W.render(start_world)
    replace = page[len("desk:"):] in start_world["pages"]

    def extra(files):
        return check(files_of(start), files, start_world["today"])[0]
    t = task(level or kind, request, start, extra, f"{page} works (visible and hidden worlds)",
             chore_keys(page, ref, replace=replace), flex={page: lambda text: isinstance(text, str)},
             extra=extra)
    return t | {"v3": kind, "chore_page": page, "hidden": hidden_worlds(empty), "check": check,
                "ref_chore": ref, "wrong_chores": list(wrong_chores), "outputs": outputs,
                "adds_mail": adds_mail}


def check_page(path, want_fn):
    def check(s, e, today):
        want = want_fn(s)
        got = e.get(path)
        if got is None or isinstance(got, str) is False:
            return False, f"{path} is missing"
        lines = [l.rstrip() for l in got.split("\n")]
        while lines and lines[-1] == "":
            lines.pop()
        return (lines == want, f"{path}: expected {len(want)} lines")
    return check


def check_sent(want_fn):
    def check(s, e, today):
        return (W.new_sent(s, e) == want_fn(s, today), "sent mails differ")
    return check


def check_number(path, value_fn):
    def check(s, e, today):
        return (one_number(value_fn(s))(e.get(path)), f"{path} should hold {value_fn(s):g}")
    return check


def no_late(w):
    for e in w["events"]:
        e["time"] = "09:00"


def no_neighbours(w):
    for c in w["contacts"]:
        if "neighbour" in c["note"]:
            c["note"] = "tennis club"


def no_invoices(w):
    for m in w["inbox"]:
        if m["subject"] == "Invoice":
            m["subject"] = "Hello"


def no_colleague_mail(w):
    for c in w["contacts"]:
        if "colleague" in c["note"]:
            c["note"] = "book club"


def no_birthdays(w):
    for c in w["contacts"]:
        if c["birthday"] == w["today"][5:]:
            c["birthday"] = "01-01" if w["today"][5:] != "01-01" else "01-02"


def nothing(w):
    pass


def late_value(s):
    return sum(1 for e in W.events(s) if e["time"] >= "18:00")


BROKEN_LATE = """-- count the events that start at 18:00 or later
-- and save the number on desk:late-count
set n to 0
for each e in pages('calendar')
  when field(e, 'time') >= 18
    set n to n + 1
  done
done
write('desk:late-count', n)"""
FIXED_LATE = BROKEN_LATE.replace("field(e, 'time') >= 18", "field(e, 'time') >= '18:00'")

BROKEN_HELLO = """-- send every contact whose note mentions neighbour a hello
for each c in pages('contacts')
  when field(c, 'note') is 'neighbour'
    send(field(c, 'mail'), 'Hello', 'Hello neighbour!')
  done
done"""
FIXED_HELLO = BROKEN_HELLO.replace("field(c, 'note') is 'neighbour'", "field(c, 'note') has 'neighbour'")

BROKEN_TOTAL = """-- add up all invoices in the inbox (subject Invoice)
-- and save the total on desk:invoice-total
set total to 0
for each m in pages('mail:inbox')
  when field(m, 'subject') is 'invoice'
    set total to total + item(words(body(m)), 2)
  done
done
write('desk:invoice-total', total)"""
FIXED_TOTAL = (BROKEN_TOTAL.replace("is 'invoice'", "is 'Invoice'")
               .replace("total + item(words(body(m)), 2)", "total + number(item(words(body(m)), 2))"))


def hello_sent(s, today):
    return sorted((m, "Hello", "Hello neighbour!") for m, c in W.contacts(s).items() if "neighbour" in c["note"])


def total_value(s):
    return sum(W.invoice_amount(m)[1] for m in W.inbox(s) if m["subject"] == "Invoice")


def with_page(world, page, text):
    w = copy.deepcopy(world)
    w["pages"][page[len("desk:"):]] = text
    return w


REF_DIGEST = """write('desk:digest-list', '')
set first to yes
for each m in pages('mail:inbox')
  set who to 'contacts:' & field(m, 'from')
  when exists(who)
    when field(who, 'note') has 'colleague'
      set line to field(m, 'from') & ' - ' & field(m, 'subject')
      when first
        write('desk:digest-list', line)
        set first to no
      otherwise
        add('desk:digest-list', line)
      done
    done
  done
done"""


def digest_lines(s):
    c = W.contacts(s)
    return [f"{m['from']} - {m['subject']}" for m in W.inbox(s) if m["from"] in c and "colleague" in c[m["from"]]["note"]]


def digest_check(s, e, today):
    got = e.get("desk:digest-list")
    if not isinstance(got, str):
        return False, "desk:digest-list is missing"
    lines = [l.rstrip() for l in got.split("\n")]
    while lines and lines[-1] == "":
        lines.pop()
    return lines == digest_lines(s), "desk:digest-list is wrong"


REF_BIRTHDAYS = """for each c in pages('contacts')
  when today() has field(c, 'birthday')
    send(field(c, 'mail'), 'Happy birthday', 'Happy birthday!')
  done
done"""


def birthday_sent(s, today):
    return sorted((m, "Happy birthday", "Happy birthday!") for m, c in W.contacts(s).items()
                  if c["birthday"] == today[5:])


REF_STATS = """write('desk:stats-out', 'mails: ' & count(pages('mail:inbox')))
add('desk:stats-out', 'contacts: ' & count(pages('contacts')))
add('desk:stats-out', 'events: ' & count(pages('calendar')))"""


def stats_lines(s):
    f = s
    return [f"mails: {len(W.inbox(f))}", f"contacts: {len(W.contacts(f))}", f"events: {len(W.events(f))}"]


CODE = {
    "fix-late": code_task(
        "fix",
        "The chore on desk:chores:late should count the events that start at 18:00 or later and "
        "save the number on desk:late-count, but it doesn't work. Fix it (keep it on "
        "desk:chores:late) and run it once.",
        "desk:chores:late", with_page(VISIBLE, "desk:chores:late", BROKEN_LATE), FIXED_LATE,
        check_number("desk:late-count", late_value), no_late, outputs=("desk:late-count",),
        wrong_chores=[("hard-coded answer", f"write('desk:late-count', {late_value(files_of(W.render(VISIBLE)))})"),
                      ("misses events at exactly 18:00", BROKEN_LATE.replace(">= 18", "> '18:00'")),
                      ("counts all events", BROKEN_LATE.replace("field(e, 'time') >= 18", "yes"))]),
    "fix-hello": code_task(
        "fix",
        "The chore on desk:chores:hello should send every contact whose note mentions neighbour a "
        "mail (subject Hello, text Hello neighbour!), but it misses some of them. Fix it (keep it on "
        "desk:chores:hello) and run it once.",
        "desk:chores:hello", with_page(VISIBLE, "desk:chores:hello", BROKEN_HELLO), FIXED_HELLO,
        check_sent(hello_sent), no_neighbours, adds_mail=True,
        wrong_chores=[("unchanged", BROKEN_HELLO),
                      ("hard-coded list", "\n".join(f"send('{m}', 'Hello', 'Hello neighbour!')" for m, c in
                                                    W.contacts(files_of(W.render(VISIBLE))).items()
                                                    if "neighbour" in c["note"])),
                      ("mails everyone", BROKEN_HELLO.replace("field(c, 'note') is 'neighbour'", "yes"))]),
    "fix-total": code_task(
        "fix",
        "The chore on desk:chores:total should add up the amounts of all invoices in the inbox "
        "(subject Invoice) and save the total on desk:invoice-total, but it doesn't work. Fix it "
        "(keep it on desk:chores:total) and run it once.",
        "desk:chores:total", with_page(VISIBLE, "desk:chores:total", BROKEN_TOTAL), FIXED_TOTAL,
        check_number("desk:invoice-total", total_value), no_invoices, outputs=("desk:invoice-total",),
        wrong_chores=[("only one bug fixed", BROKEN_TOTAL.replace("total + item(words(body(m)), 2)",
                                                                  "total + number(item(words(body(m)), 2))")),
                      ("hard-coded total", f"write('desk:invoice-total', {total_value(files_of(W.render(VISIBLE))):g})")]),
    "write-digest": code_task(
        "write",
        "Write a chore on the page desk:chores:digest that makes the page desk:digest-list with one "
        "line for every mail in the inbox from a contact whose note mentions colleague, written "
        "sender - subject (e.g. ana - Lunch?), in the order pages('mail:inbox') gives. If there are "
        "no such mails, desk:digest-list must still be made, empty. Run it once. It will also be "
        "tested on other desks.",
        "desk:chores:digest", VISIBLE, REF_DIGEST, digest_check, no_colleague_mail,
        outputs=("desk:digest-list",),
        wrong_chores=[("add only (no page when nothing matches)", REF_DIGEST.replace(
                          "write('desk:digest-list', '')\n", "")),
                      ("hard-coded lines", "write('desk:digest-list', '" + digest_lines(files_of(W.render(VISIBLE)))[0] + "')"),
                      ("note is colleague exactly", REF_DIGEST.replace("has 'colleague'", "is 'colleague'"))]),
    "write-birthdays": code_task(
        "write",
        "Write a chore on the page desk:chores:birthdays that sends every contact whose birthday is "
        "today a mail with the subject Happy birthday and the text Happy birthday!  Run it once. "
        "It will also be tested on other desks and other dates.",
        "desk:chores:birthdays", VISIBLE, REF_BIRTHDAYS, check_sent(birthday_sent), no_birthdays,
        adds_mail=True,
        wrong_chores=[("hard-coded date", REF_BIRTHDAYS.replace("today() has field(c, 'birthday')",
                                                                "field(c, 'birthday') is '11-12'")),
                      ("hard-coded people", "\n".join(f"send('{m}', 'Happy birthday', 'Happy birthday!')"
                                                      for m, c in W.contacts(files_of(W.render(VISIBLE))).items()
                                                      if c["birthday"] == W.TODAY[5:]))]),
    "write-stats": code_task(
        "write",
        "Write a chore on the page desk:chores:stats that writes the page desk:stats-out with exactly "
        "three lines: mails: N (the number of mails in the inbox), contacts: N and events: N (the "
        "number of events). Run it once. It will also be tested on other desks.",
        "desk:chores:stats", VISIBLE, REF_STATS, check_page("desk:stats-out", stats_lines), nothing,
        outputs=("desk:stats-out",),
        wrong_chores=[("hard-coded numbers", "write('desk:stats-out', 'mails: 150')\nadd('desk:stats-out', 'contacts: 100')\n"
                                             "add('desk:stats-out', 'events: 80')"),
                      ("counts sent instead of inbox", REF_STATS.replace("pages('mail:inbox')", "pages('mail:sent')"))]),
}

TASKS_V3 = {**COUNT, **BULK, **CODE}

# ================================================================= practice (calibration)
# Only for tuning (never part of the experiment), on a different world so practice games
# can't teach the test world's contents.

PRACTICE_WORLD = W.generate(SEED + 999)


def oracle_photos(start):
    return sum(1 for m in W.inbox(files_of(start)) if m["subject"] == "Photos")


REF_PHOTOS = """set n to 0
for each m in pages('mail:inbox')
  when field(m, 'subject') is 'Photos'
    set n to n + 1
  done
done
write('desk:answer', n)"""


def oracle_morning(start):
    return sum(1 for e in W.events(files_of(start)) if e["date"].startswith("2026-11") and e["time"] < "12:00")


REF_MORNING = """set n to 0
for each e in pages('calendar')
  when field(e, 'date') < '2026-12-01' and field(e, 'time') < '12:00'
    set n to n + 1
  done
done
write('desk:answer', n)"""

MOVING = "I am moving house next month!"


def expect_moving(s):
    return sorted((m, "Moving", MOVING) for m, c in W.contacts(s).items() if c["city"] == "Dunmore")


REF_MOVING = f"""for each c in pages('contacts')
  when field(c, 'city') is 'Dunmore'
    send(field(c, 'mail'), 'Moving', '{MOVING}')
  done
done"""

BROKEN_FAMILY = """-- list the names of contacts whose note mentions family
write('desk:family', '')
for each c in pages('contact')
  when field(c, 'note') has 'family'
    add('desk:family', field(c, 'name'))
  done
done"""
FIXED_FAMILY = BROKEN_FAMILY.replace("pages('contact')", "pages('contacts')")


def family_check(s, e, today):
    got = e.get("desk:family")
    if not isinstance(got, str):
        return False, "desk:family is missing"
    want = [c["name"] for c in W.contacts(s).values() if "family" in c["note"]]
    lines = [l.rstrip() for l in got.split("\n") if l.strip()]
    return lines == want, "desk:family is wrong"


def no_family(w):
    for c in w["contacts"]:
        if "family" in c["note"]:
            c["note"] = "tennis club"


REF_ASHFORD = """set n to 0
for each c in pages('contacts')
  when field(c, 'city') is 'Ashford'
    set n to n + 1
  done
done
write('desk:city-count', n)"""


def ashford_value(s):
    return sum(1 for c in W.contacts(s).values() if c["city"] == "Ashford")


def no_ashford(w):
    for c in w["contacts"]:
        if c["city"] == "Ashford":
            c["city"] = "Dunmore"


PRACTICE_V3 = {
    "p3-photos": answer_task(
        "count", "How many mails in your inbox have the subject Photos? Save just the number on the "
        "page desk:answer. " + SCRATCH, REF_PHOTOS, oracle_photos, world=PRACTICE_WORLD),
    "p3-morning": answer_task(
        "count", "How many events in November 2026 start before 12:00? Save just the number on the "
        "page desk:answer. " + SCRATCH, REF_MORNING, oracle_morning, world=PRACTICE_WORLD),
    "p3-moving": bulk_task(
        "Send every contact who lives in Dunmore a mail with the subject Moving and the text: "
        f"{MOVING} " + SCRATCH, REF_MOVING, expect_sent=expect_moving, world=PRACTICE_WORLD),
    "p3-fix-family": code_task(
        "fix", "The chore on desk:chores:family should write the names of the contacts whose note "
        "mentions family on the page desk:family (one per line), but it doesn't work. Fix it (keep "
        "it on desk:chores:family) and run it once.",
        "desk:chores:family", with_page(PRACTICE_WORLD, "desk:chores:family", BROKEN_FAMILY), FIXED_FAMILY,
        family_check, no_family, outputs=("desk:family",),
        wrong_chores=[("unchanged", BROKEN_FAMILY)]),
    "p3-write-ashford": code_task(
        "write", "Write a chore on the page desk:chores:ashford that saves the number of contacts who "
        "live in Ashford on the page desk:city-count. Run it once. It will also be tested on other "
        "desks.",
        "desk:chores:ashford", PRACTICE_WORLD, REF_ASHFORD, check_number("desk:city-count", ashford_value),
        no_ashford, outputs=("desk:city-count",),
        wrong_chores=[("hard-coded", f"write('desk:city-count', {ashford_value(files_of(W.render(PRACTICE_WORLD)))})")]),
}
