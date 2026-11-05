"""The big start world for experiment 3, made by a seeded generator.

A world is plain data (contacts, inbox, sent, events, extra pages) that is
rendered into start files. The same generator with other seeds (and small
changes) makes the hidden worlds that players' chores are re-run on.

Oracle helpers at the bottom read a world back from a files dict (the
DUMP / parse_start format) so the checkers never depend on generator
internals: they compute the right answer from what is really on the desk.
"""

import random
import re

TODAY = "2026-11-12"  # a Thursday

FIRST = """ada alan alba amir ana anton arne astrid ben bo carla cleo dan dina edgar eli ella
emil eva felix finn frida gina greta hana hanna hugo ida ines ivan jo jonas juno kai karl kira
lars leif lina luca lucy maja mara marco mia mila mira nadia nico nina noah nora olaf olga omar
oscar paula pia priya quinn rafael rosa ruth sami sara selma sofia stan sven tara teo thea tilda
uma uri vera viktor walt wanda xena yara yusuf zara aldo bea cyril dora elif fabio goran hilde
ilse jakob kasper liv magnus nell odile petra rune""".split()
LAST = """Costa Smith Berg Lee Haddad Novak Silva Weber Rossi Jensen Kowal Moreau Okafor Tanaka Ito
Park Nagy Horvat Dubois Meyer Fischer Laine Santos Duarte Brandt Keller Lund Moss Reed Varga""".split()
CITIES = ["Ashford", "Brookville", "Cedar Hill", "Dunmore"]
NOTES = (["neighbour"] * 4 + ["neighbour, has a dog"] * 3 + ["neighbour, has a cat"] * 2
         + ["lives across the street, neighbour"] + ["colleague"] * 4 + ["colleague, sits near me"] * 2
         + ["old colleague from the bank"] + ["plumber", "piano teacher", "old school friend",
         "works at the bakery", "tennis club", "book club", "family", "dentist", "cousin",
         "met at the market", "family, my aunt", "book club, likes crime novels", "tennis club"])
STRANGERS = ["shop", "bank", "news", "zoe", "kim", "max", "rex", "tom"]
SUBJECTS = (["Invoice"] * 6 + ["Hello", "Lunch?", "Photos", "Question", "Meeting", "Reminder",
            "Party", "Book club", "News", "Weekend"])
TITLES = ["Dentist", "Lunch", "Team call", "Piano lesson", "Book club", "Gym", "Haircut", "Dinner",
          "Coffee", "Tennis", "Movie", "Market"]
TIMES = ["08:00", "09:00", "09:30", "10:00", "12:00", "12:30", "14:00", "16:00", "17:30", "18:00",
         "18:30", "19:00", "20:00"]
BODIES = ["Hi, how are you? Let me know if you have time soon.",
          "Here is what we talked about. See you next week.",
          "Can you call me when you have a minute?",
          "Thanks for last time, it was really nice.",
          "Just a short note to say hello."]
assert len(set(FIRST)) == len(FIRST) >= 100 and not set(FIRST) & set(STRANGERS)


def date_add(date, days):
    import datetime as dt
    return (dt.date.fromisoformat(date) + dt.timedelta(days=days)).isoformat()


def generate(seed, n_contacts=100, n_mails=150, n_sent=20, n_events=80, today=TODAY,
             first="2026-11-01", last="2026-12-31"):
    r = random.Random(seed)
    names = r.sample(FIRST, n_contacts)
    contacts = []
    for i, n in enumerate(names):
        contacts.append({"mail": n, "name": f"{n.capitalize()} {r.choice(LAST)}",
                         "phone": f"555-{1000 + r.randrange(9000)}", "city": r.choice(CITIES),
                         "birthday": f"{r.randint(1, 12):02d}-{r.randint(1, 28):02d}",
                         "note": r.choice(NOTES)})
    span = (int(last[8:]) + 31 * (int(last[5:7]) - int(first[5:7]))) - int(first[8:])
    inbox = []
    for _ in range(n_mails):
        sender = r.choice(names) if r.random() < 0.88 or not STRANGERS else r.choice(STRANGERS)
        subject = r.choice(SUBJECTS)
        if subject == "Invoice":
            amount = r.randint(15, 300) + (0.5 if r.random() < 0.3 else 0)
            amt = f"{amount:g}"
            body = f"Total: {amt} euro. Please pay before {date_add(today, r.randint(5, 30))}."
        else:
            body = r.choice(BODIES)
        inbox.append({"from": sender, "subject": subject, "body": body})
    sent = [{"to": r.choice(names), "subject": r.choice(["Hello", "Re: Question", "Photos", "Lunch?"]),
             "body": r.choice(BODIES)} for _ in range(n_sent)]
    events = []
    for _ in range(n_events):
        while True:
            d = date_add(first, r.randint(0, 61))
            if first <= d <= last:
                break
        roll = r.random()
        with_ = r.choice(names) if roll < 0.55 else ("" if roll < 0.8 else r.choice(STRANGERS[3:]))
        events.append({"date": d, "time": r.choice(TIMES), "title": r.choice(TITLES), "with": with_})
    return {"today": today, "contacts": contacts, "inbox": inbox, "sent": sent, "events": events,
            "pages": {}, "drawers": ["chores"]}


def render(w):
    out = [f"#today {w['today']}"]
    for d in w["drawers"]:
        out.append(f"#drawer {d}")
    for c in w["contacts"]:
        out += [f"#page contacts:{c['mail']}", f"name: {c['name']}", f"mail: {c['mail']}",
                f"phone: {c['phone']}", f"city: {c['city']}", f"birthday: {c['birthday']}",
                f"note: {c['note']}", "#end"]
    for i, m in enumerate(w["inbox"], 1):
        out += [f"#page mail:inbox:m{i}", f"from: {m['from']}", "to: me", f"subject: {m['subject']}", "",
                m["body"], "#end"]
    for i, m in enumerate(w["sent"], 1):
        out += [f"#page mail:sent:s{i}", "from: me", f"to: {m['to']}", f"subject: {m['subject']}", "",
                m["body"], "#end"]
    for i, e in enumerate(w["events"], 1):
        out += [f"#page calendar:e{i}", f"date: {e['date']}", f"time: {e['time']}", f"title: {e['title']}",
                f"with: {e['with']}", "#end"]
    for path, text in w["pages"].items():
        out += [f"#page {path}", text.rstrip("\n"), "#end"]
    return "\n".join(out) + "\n"


# ---- reading a world back from files (for the checkers) -------------------------------

def _num(name):
    m = re.search(r"(\d+)$", name)
    return (int(m.group(1)) if m else 0, name)


def header(text):
    fields = {}
    for line in text.split("\n"):
        if line == "":
            break
        k, sep, v = line.partition(":")
        if sep:
            fields.setdefault(k.strip().lower(), v.strip())
    return fields


def body(text):
    return text.split("\n\n", 1)[1] if "\n\n" in text else ""


def pages_in(files, drawer):
    """Pages directly in a drawer, in the chores' natural order (m2 before m10)."""
    pre = f"desk:{drawer}:"
    names = [k[len(pre):] for k, v in files.items() if k.startswith(pre) and ":" not in k[len(pre):]
             and isinstance(v, str)]
    return [pre + n for n in sorted(names, key=_num)]


def contacts(files):
    return {header(files[p]).get("mail", ""): header(files[p]) | {"path": p} for p in pages_in(files, "contacts")}


def inbox(files):
    return [header(files[p]) | {"body": body(files[p]), "path": p} for p in pages_in(files, "mail:inbox")]


def sent(files):
    return [header(files[p]) | {"body": body(files[p]), "path": p} for p in pages_in(files, "mail:sent")]


def events(files):
    return [header(files[p]) | {"path": p} for p in pages_in(files, "calendar")]


def invoice_amount(mail):
    """The amount of an invoice mail ('Total: 120.5 euro ...'), as text and as a number."""
    words = mail["body"].split()
    return words[1], float(words[1])


def new_sent(start, end):
    """Sent mails added during the game, as a sorted list of (to, subject, body)."""
    def key(m):
        return (m.get("to", ""), m.get("subject", ""), m["body"].rstrip())
    before = [key(m) for m in sent(start)]
    after = [key(m) for m in sent(end)]
    for k in before:
        if k in after:
            after.remove(k)
    return sorted(after)


def new_events(start, end):
    def key(e):
        return (e.get("date", ""), e.get("time", ""), e.get("title", ""), e.get("with", ""))
    before = [key(e) for e in events(start)]
    after = [key(e) for e in events(end)]
    for k in before:
        if k in after:
            after.remove(k)
    return sorted(after)


def weekday(date):
    import datetime as dt
    return dt.date.fromisoformat(date).strftime("%A").lower()
