"""Version 2 tasks: they need several apps together (Mail, Contacts, Calendar,
Settings, Calculator, plus pages). TEST tasks are for experiment 2; PRACTICE
tasks are only for tuning (action budget, wording) and never used in it.

Every task starts from the same small world (BASE) plus its own extras.
Free-text results (mails, saved notes) are checked with `flex` tests; the rest
of the end state must match the reference solution exactly (calendar events
are compared by content, not by their page number).
"""

import re

from tasks import norm, task

BASE = """
#page contacts:ana
name: Ana Costa
mail: ana
phone: 555-0303
note: neighbour, has a cat
#end
#page contacts:jo
name: Jo Smith
mail: jo
phone: 555-0606
note: plumber
#end
#page contacts:lina
name: Lina Berg
mail: lina
phone: 555-0505
note: piano teacher
#end
#page contacts:mira
name: Mira Lee
mail: mira
phone: 555-0101
note: old school friend, likes mint tea
#end
#page contacts:omar
name: Omar Haddad
mail: omar
phone: 555-0404
note: works at the bakery
#end
#page calendar:e1
date: 2026-11-10
time: 09:00
title: Dentist
with:
#end
#page calendar:e2
date: 2026-11-14
time: 18:00
title: Piano lesson
with: lina
#end
#page calendar:e3
date: 2026-11-20
time: 12:00
title: Lunch with Ana
with: ana
#end
#page mail:inbox:m1
from: omar
to: me
subject: Bakery closed

Hi Sam, the bakery is closed on 2026-11-13. See you after that! Omar
#end
#page lists:shopping
eggs
bread
#end
#page welcome
Hello Sam. Your mail, contacts and calendar are waiting for you.
#end
"""


def world(extra=""):
    return BASE + extra


def inbox(n, sender, subject, body):
    return f"#page mail:inbox:m{n}\nfrom: {sender}\nto: me\nsubject: {subject}\n\n{body}\n#end\n"


# ---- helpers for checks ---------------------------------------------------------

def mail_parts(text):
    if not isinstance(text, str):
        return None
    head, _, body = text.partition("\n\n")
    fields = {}
    for line in head.splitlines():
        k, _, v = line.partition(":")
        fields[k.strip().lower()] = v.strip()
    return {"to": fields.get("to", ""), "subject": fields.get("subject", ""), "body": body}


def has_word(text, word):
    """`word` appears in text as a whole word/number (case doesn't matter)."""
    return bool(re.search(rf"(?<![\w.]){re.escape(word)}(?![\w]|\.\d)", text, re.I))


def sent(to, words=(), subject=None, nonempty=False):
    """Flex test for a sent mail."""
    def test(text):
        m = mail_parts(text)
        if not m or m["to"] != to:
            return False
        if subject is not None and m["subject"].lower() != subject.lower():
            return False
        if nonempty and not m["body"].strip():
            return False
        return all(any(has_word(m["subject"] + "\n" + m["body"], w) for w in (ws if isinstance(ws, tuple) else (ws,)))
                   for ws in words)
    return test


MONTHS = ["january", "february", "march", "april", "may", "june", "july", "august", "september",
          "october", "november", "december"]


def body(text):
    m = mail_parts(text)
    return m["body"] if m else ""


def has_amount(text, value):
    """The amount appears as a number (163, 163.00, 163,00 all count; -163 does not)."""
    for minus, n in re.findall(r"(?<![\w.,])(-\s*)?(\d+(?:[.,]\d+)?)(?![\w]|[.,]\d)", text):
        if not minus and float(n.replace(",", ".")) == value:
            return True
    return False


def mentions_date(text, y, m, d):
    """The date appears in a common form: 2026-11-14, 14 November, Nov 14, 14/11, 11/14 ..."""
    t = text.lower()
    full, short = MONTHS[m - 1], MONTHS[m - 1][:3]
    year = rf"(?:,?\s*{y}\b)?(?!,?\s*\d)"  # a year, if written, must be the right one
    forms = [rf"(?<![\d-]){y}-{m:02d}-{d:02d}(?!\d)",
             rf"(?<!\d){d}(st|nd|rd|th)?\s+(of\s+)?({full}|{short})\b{year}",
             rf"\b({full}|{short})\.?\s+{d}(st|nd|rd|th)?\b{year}",
             rf"(?<![\d/.-])0?{d}[/.-]0?{m}(?:[/.-](?:{y}|{y % 100}))?(?![/.-]?\d)",
             rf"(?<![\d/.-])0?{m}/0?{d}(?:/(?:{y}|{y % 100}))?(?![/.-]?\d)"]
    return any(re.search(f, t) for f in forms)


def mentions_time(text, h, mi):
    """The time appears in a common form: 18:00, 18.00, 6 pm, 6:00pm, 18h ..."""
    t = text.lower()
    h12, ampm = (h % 12 or 12), ("pm" if h >= 12 else "am")
    no_ampm = r"(?!\s*[ap]\.?m\b)"  # a 24-hour time must not carry am/pm
    forms = [rf"(?<![\d:.]){h:02d}[:.]{mi:02d}(?!\d){no_ampm}", rf"(?<![\d:.]){h}[:.]{mi:02d}(?!\d){no_ampm}",
             rf"(?<![\d:.]){h12}[:.]{mi:02d}\s*{ampm}\b"]
    if mi == 0:
        forms += [rf"(?<![\d:.]){h12}\s*{ampm}\b", rf"(?<![\d:.]){h}\s*h\b", rf"(?<![\d:.]){h}\s*o'?clock\b"]
    return any(re.search(f, t) for f in forms)


def starts_with_yes(text):
    return bool(re.match(r"\s*yes\b", text, re.I))


def mail_to(to, test=lambda b, m: True, subject=None):
    """Flex test: a sent mail to `to` whose body (and subject) pass `test(body, parts)`."""
    def check(text):
        m = mail_parts(text)
        if not m or m["to"] != to:
            return False
        if subject is not None and m["subject"].strip().lower() != subject.lower():
            return False
        return test(m["body"], m)
    return check


OMAR_LINE = "Hi Sam, the bakery is closed on 2026-11-13. See you after that! Omar"


def has_line(text, line):
    squash = lambda x: " ".join(x.split()).lower()  # noqa: E731
    return isinstance(text, str) and squash(line) in squash(text)


def mail_text(to, subject, text):
    return f"from: me\nto: {to}\nsubject: {subject}\n\n{text}"


def event(f, date, time, title, with_=""):
    want = (date, time, title, with_)
    for k, v in f.items():
        if k.startswith("desk:calendar:") and isinstance(v, str):
            got = tuple(next((l.split(":", 1)[1].strip() for l in v.splitlines() if l.startswith(key + ":")), "")
                        for key in ("date", "time", "title", "with"))
            if got == want:
                return True
    return False


def sent_count(f):
    return sum(1 for k, v in f.items() if k.startswith("desk:mail:sent:") and isinstance(v, str))


def page_has(*words):
    def test(text):
        return isinstance(text, str) and all(w.lower() in text.lower() for w in words)
    return test


S1 = "desk:mail:sent:s1"

TASKS_V2 = {
    "dinner": task(
        "v2",
        "Mira invited you to dinner (see your mail). Add the dinner to the calendar with the title "
        "Dinner with Mira and with set to mira, then reply to Mira; start your reply with the word Yes.",
        world(inbox(2, "mira", "Dinner?", "Hi Sam! Can we have dinner on 2026-11-12 at 19:30? Mira")),
        lambda f: event(f, "2026-11-12", "19:30", "Dinner with Mira", "mira") and mail_to("mira", lambda b, m: starts_with_yes(b))(f.get(S1)),
        "event 2026-11-12 19:30 'Dinner with Mira' with mira; one reply to mira starting with Yes",
        "mail{enter}{down}{enter}rYes, see you then!{ctrl+k}{esc}"
        "calendar{enter}a2026-11-12{down}19:30{down}Dinner with Mira{down}mira{ctrl+k}{esc}",
        flex={S1: mail_to("mira", lambda b, m: starts_with_yes(b))},
        right=[(S1, mail_text("mira", "Re: Dinner?", "yes! 19:30 works."))],
        wrong=[(S1, mail_text("mira", "Re: Dinner?", "No, I cannot say yes.")),
               (S1, mail_text("ana", "Re: Dinner?", "Yes, see you then!"))],
    ),
    "plumber-number": task(
        "v2",
        "Ana asked for a phone number (see your mail). Reply to her with that number.",
        world(inbox(2, "ana", "Plumber?", "Hi Sam, my sink is leaking. What is the phone number of your plumber? Ana")),
        lambda f: mail_to("ana", lambda b, m: bool(re.search(r"(?<!\d)555[- ]?0606(?!\d)", b)))(f.get(S1)),
        "one reply to ana containing Jo's number 555-0606",
        "mail{enter}{down}{enter}rThe plumber is Jo Smith: 555-0606{ctrl+k}{esc}",
        flex={S1: mail_to("ana", lambda b, m: bool(re.search(r"(?<!\d)555[- ]?0606(?!\d)", b)))},
        right=[(S1, mail_text("ana", "Re: Plumber?", "Call Jo on 555 0606."))],
        wrong=[(S1, mail_text("ana", "Re: Plumber?", "Call Jo on 555-0404.")),
               (S1, mail_text("ana", "Re: Plumber?", "Jo is the plumber."))],
    ),
    "new-neighbour": task(
        "v2",
        "Add the person who sent the newest mail to your contacts, with their name, their mail "
        "address and their phone number (leave the note empty).",
        world(inbox(2, "pia", "Hello neighbour",
                    "Hi! I'm Pia Novak, I just moved in next door.\nMy phone number is 555-0707. Pia")),
        lambda f: norm(f.get("desk:contacts:pia")) == ["name: Pia Novak", "mail: pia", "phone: 555-0707", "note:"],
        "contact pia: Pia Novak, 555-0707, empty note",
        "contacts{enter}aPia Novak{down}pia{down}555-0707{ctrl+k}{esc}",
    ),
    "move-lesson": task(
        "v2",
        "Lina asked to move a lesson (see your mail). Change it in the calendar, then reply to Lina "
        "with the new date.",
        world(inbox(2, "lina", "Lesson", "Hi Sam, can we move your piano lesson to 2026-11-15? Same time. Lina")),
        lambda f: event(f, "2026-11-15", "18:00", "Piano lesson", "lina") and not event(f, "2026-11-14", "18:00", "Piano lesson", "lina")
        and mail_to("lina", lambda b, m: mentions_date(b, 2026, 11, 15))(f.get(S1)),
        "piano lesson moved to 2026-11-15 18:00; one reply to lina with the new date",
        "calendar{enter}{down}{enter}{bksp*10}2026-11-15{ctrl+k}{esc}"
        "mail{enter}{down}{enter}rDone, see you on 2026-11-15!{ctrl+k}{esc}",
        flex={S1: mail_to("lina", lambda b, m: mentions_date(b, 2026, 11, 15))},
        right=[(S1, mail_text("lina", "Re: Lesson", "Moved to November 15th, same time."))],
        wrong=[(S1, mail_text("lina", "Re: Lesson", "I refuse to change your lesson.")),
               (S1, mail_text("lina", "Re: Lesson", "See you on 2026-11-14."))],
    ),
    "cancel-lunch": task(
        "v2",
        "Ana cancelled a plan (see your mail). Remove it from the calendar.",
        world(inbox(2, "ana", "Lunch", "Sorry Sam, I can't make our lunch. Let's skip it this time. Ana")),
        lambda f: not event(f, "2026-11-20", "12:00", "Lunch with Ana", "ana") and event(f, "2026-11-10", "09:00", "Dentist"),
        "the lunch with Ana is gone, other events stay",
        "calendar{enter}{down}{down}t{ctrl+k}{esc}",
    ),
    "setup-sam": task(
        "v2",
        "Do what the page todo asks.",
        world("#page todo\nSet up this computer for Sam:\n- name: Sam\n- theme: black\n- clock: 12h\n#end\n"),
        lambda f: norm(f.get("desk:system:settings")) == ["name: Sam", "theme: black", "start: shell", "clock: 12h"],
        "settings: name Sam, theme black, clock 12h, start unchanged",
        "settings{enter}{enter}{bksp*6}Sam{enter}{down}{right}{down}{down}{right}{ctrl+k}{esc}",
    ),
    "invoice": task(
        "v2",
        "Jo sent an invoice (see your mail). Work out the total amount and reply to Jo with it.",
        world(inbox(2, "jo", "Invoice", "Hi Sam, for fixing the sink: 3 hours at 45 euro per hour,\nplus 28 euro for parts. Jo")),
        lambda f: mail_to("jo", lambda b, m: has_amount(b, 163))(f.get(S1)),
        "one reply to jo containing the total 163",
        "mail{enter}{down}{enter}rThe total is 163 euro.{ctrl+k}{esc}",
        flex={S1: mail_to("jo", lambda b, m: has_amount(b, 163))},
        right=[(S1, mail_text("jo", "Re: Invoice", "Total: 163.00 euro")),
               (S1, mail_text("jo", "Re: Invoice", "3 x 45 + 28 = 163"))],
        wrong=[(S1, mail_text("jo", "Re: Invoice", "-163 euro")),
               (S1, mail_text("jo", "Re: Invoice", "The total is 1630 euro.")),
               (S1, mail_text("jo", "Re: Invoice", "- 163 euro")),
               (S1, mail_text("jo", "Re: Invoice", "Thanks Jo!"))],
    ),
    "forward-bakery": task(
        "v2",
        "Forward Omar's mail about the bakery to Mira.",
        world(),
        lambda f: mail_to("mira", lambda b, m: has_line(b, OMAR_LINE))(f.get(S1)),
        "one mail to mira containing Omar's whole message",
        "mail{enter}{enter}fmira{ctrl+k}{esc}",
        flex={S1: mail_to("mira", lambda b, m: has_line(b, OMAR_LINE))},
        right=[(S1, mail_text("mira", "Fwd: Bakery closed", "FYI:\n" + OMAR_LINE))],
        wrong=[(S1, mail_text("mira", "Fwd: Bakery closed", "The bakery is closed on 2026-11-13.")),
               (S1, mail_text("ana", "Fwd: Bakery closed", OMAR_LINE))],
    ),
    "birthday": task(
        "v2",
        "One of your contacts has a birthday written in their note. Add it to the calendar at 08:00, "
        "with the title Birthday followed by their first name (for example Birthday Ana) and with "
        "set to their mail address.",
        world("#page contacts:lina\nname: Lina Berg\nmail: lina\nphone: 555-0505\nnote: piano teacher, birthday on 2026-11-22\n#end\n"),
        lambda f: event(f, "2026-11-22", "08:00", "Birthday Lina", "lina"),
        "event 2026-11-22 08:00 'Birthday Lina' with lina",
        "calendar{enter}a2026-11-22{down}08:00{down}Birthday Lina{down}lina{ctrl+k}{esc}",
    ),
    "week-list": task(
        "v2",
        "Make a page called week on the desk listing the titles of all calendar events from "
        "2026-11-10 to 2026-11-16, one title per line, in date order.",
        world("#page calendar:e4\ndate: 2026-11-12\ntime: 16:00\ntitle: Haircut\nwith:\n#end\n"),
        lambda f: norm(f.get("desk:week")) == ["Dentist", "Haircut", "Piano lesson"],
        "desk:week = Dentist / Haircut / Piano lesson",
        "write week Dentist{enter}add week Haircut{enter}add week Piano lesson{enter}",
    ),
    "omar-phone": task(
        "v2",
        "Omar has a new phone number (see your mail). Update his contact.",
        world(inbox(2, "omar", "New number", "Hi Sam, I have a new phone number: 555-0999. Omar")),
        lambda f: norm(f.get("desk:contacts:omar")) == ["name: Omar Haddad", "mail: omar", "phone: 555-0999", "note: works at the bakery"],
        "contact omar has phone 555-0999, rest unchanged",
        "contacts{enter}{down*4}{enter}{down}{down}{end}{bksp*8}555-0999{ctrl+k}{esc}",
    ),
    "party": task(
        "v2",
        "Send a mail to Mira and a separate mail to Ana, both with the subject Party and the text: "
        "Party at my place on Saturday at 20:00!",
        world(),
        lambda f: sorted(mail_parts(f.get(p))["to"] for p in (S1, "desk:mail:sent:s2") if f.get(p)) == ["ana", "mira"],
        "two mails, one to mira and one to ana, subject Party, with the party text",
        "mail{enter}wmira{tab}Party{tab}Party at my place on Saturday at 20:00!{ctrl+k}"
        "wana{tab}Party{tab}Party at my place on Saturday at 20:00!{ctrl+k}{esc}",
        flex={p: (lambda t: bool(mail_parts(t)) and mail_parts(t)["to"] in ("mira", "ana")
                  and mail_parts(t)["subject"].lower() == "party"
                  and "party at my place on saturday at 20:00" in mail_parts(t)["body"].lower())
              for p in (S1, "desk:mail:sent:s2")},
        extra=lambda f: sorted(mail_parts(f.get(p))["to"] for p in (S1, "desk:mail:sent:s2") if f.get(p)) == ["ana", "mira"],
    ),
    "haircut-plan": task(
        "v2",
        "Do what the page plans says. Use the title Haircut and with set to ana for the event, and put "
        "the date and time in the mail.",
        world("#page plans\nBook a haircut on 2026-11-18 at 10:00,\nand tell Ana by mail: she wants to come too.\n#end\n"),
        lambda f: event(f, "2026-11-18", "10:00", "Haircut", "ana")
        and mail_to("ana", lambda b, m: mentions_date(m["subject"] + " " + b, 2026, 11, 18) and mentions_time(m["subject"] + " " + b, 10, 0))(f.get(S1)),
        "event 2026-11-18 10:00 Haircut with ana; one mail to ana with the date and time",
        "calendar{enter}a2026-11-18{down}10:00{down}Haircut{down}ana{ctrl+k}{esc}"
        "mail{enter}wana{tab}Haircut{tab}Haircut on 2026-11-18 at 10:00, join me!{ctrl+k}{esc}",
        flex={S1: mail_to("ana", lambda b, m: mentions_date(m["subject"] + " " + b, 2026, 11, 18)
                          and mentions_time(m["subject"] + " " + b, 10, 0))},
        right=[(S1, mail_text("ana", "Haircut", "Nov 18 at 10am, come along!"))],
        wrong=[(S1, mail_text("ana", "Haircut", "Hello.")),
               (S1, mail_text("ana", "Haircut", "Haircut on 2026-11-18, come along!")),
               (S1, mail_text("ana", "Haircut", "2026-11-18 at 10:00 pm")),
               (S1, mail_text("ana", "Haircut", "18 November 2025 at 10:00"))],
    ),
    "jo-retired": task(
        "v2",
        "Jo has retired (see your mail). Remove Jo from your contacts and throw away every mail from Jo.",
        world(inbox(2, "jo", "Retiring", "Hi Sam, I am retiring, so please use another plumber from now on. Thanks! Jo")),
        lambda f: "desk:contacts:jo" not in f and "desk:mail:inbox:m2" not in f and "desk:mail:inbox:m1" in f,
        "contact jo and the mail from jo are gone; Omar's mail stays",
        "contacts{enter}{down}t{ctrl+k}{esc}mail{enter}{down}t{ctrl+k}{esc}",
    ),
    "price-total": task(
        "v2",
        "Mail Mira the total of the prices on the page prices in the lists drawer, with the subject Total.",
        world("#page lists:prices\neggs 3.20\nbread 2.50\ntea 4.75\n#end\n"),
        lambda f: mail_to("mira", lambda b, m: has_amount(b, 10.45), subject="Total")(f.get(S1)),
        "one mail to mira, subject Total, containing 10.45",
        "mail{enter}wmira{tab}Total{tab}The total is 10.45{ctrl+k}{esc}",
        flex={S1: mail_to("mira", lambda b, m: has_amount(b, 10.45), subject="Total")},
        right=[(S1, mail_text("mira", "Total", "10,45 euro")), (S1, mail_text("mira", "total", "10.450"))],
        wrong=[(S1, mail_text("mira", "Total", "10.4")), (S1, mail_text("mira", "Total", "-10.45")),
               (S1, mail_text("mira", "Prices", "10.45"))],
    ),
    "who-called": task(
        "v2",
        "Find out who called (see the page missed) and write their full name, as it is in Contacts, "
        "into a new page called caller on the desk.",
        world("#page missed\nMissed call at 14:05 from 555-0404.\n#end\n"),
        lambda f: norm(f.get("desk:caller")) == ["Omar Haddad"],
        "desk:caller = Omar Haddad",
        "write caller Omar Haddad{enter}",
    ),
    "start-mail": task(
        "v2",
        "Make the computer open the Mail app when it starts, and set the name to Sam.",
        world(),
        lambda f: norm(f.get("desk:system:settings")) == ["name: Sam", "theme: blue", "start: mail", "clock: 24h"],
        "settings: name Sam, start mail, rest unchanged",
        "settings{enter}{enter}{bksp*6}Sam{enter}{down}{down}{right}{right}{ctrl+k}{esc}",
    ),
    "lesson-time": task(
        "v2",
        "Mail Lina the date and time of the piano lesson in your calendar (any subject).",
        world(),
        lambda f: mail_to("lina", lambda b, m: mentions_date(m["subject"] + " " + b, 2026, 11, 14)
                          and mentions_time(m["subject"] + " " + b, 18, 0))(f.get(S1)),
        "one mail to lina with the date 2026-11-14 and the time 18:00",
        "mail{enter}wlina{tab}Piano{tab}The lesson is on 2026-11-14 at 18:00.{ctrl+k}{esc}",
        flex={S1: mail_to("lina", lambda b, m: mentions_date(m["subject"] + " " + b, 2026, 11, 14)
                          and mentions_time(m["subject"] + " " + b, 18, 0))},
        right=[(S1, mail_text("lina", "Lesson", "November 14, 2026 at 6pm")),
               (S1, mail_text("lina", "Lesson", "2026-11-14, 6 pm"))],
        wrong=[(S1, mail_text("lina", "Lesson", "2026-11-15 at 18:00")),
               (S1, mail_text("lina", "Lesson", "See you soon!")),
               (S1, mail_text("lina", "Lesson", "14 November 2027 at 18:00")),
               (S1, mail_text("lina", "Lesson", "2026-11-140 at 18:00")),
               (S1, mail_text("lina", "Lesson", "2026-11-14 at 18:00 am")),
               (S1, mail_text("lina", "Lesson", "2026-11-14 at 6 am"))],
    ),
    "mira-likes": task(
        "v2",
        "Add what Mira likes (it is in her contact note) as a new last line of the page shopping in "
        "the lists drawer.",
        world(),
        lambda f: norm(f.get("desk:lists:shopping")) == ["eggs", "bread", "mint tea"],
        "lists:shopping = eggs / bread / mint tea",
        "add lists:shopping mint tea{enter}",
    ),
    "keep-bakery": task(
        "v2",
        "Save Omar's bakery mail as a page called bakery inside a new drawer called notes, then throw "
        "the mail away.",
        world(),
        lambda f: has_line(f.get("desk:notes:bakery"), OMAR_LINE) and "desk:mail:inbox:m1" not in f,
        "notes:bakery holds Omar's whole message; the mail is gone",
        "build notes{enter}copy mail:inbox:m1 notes:bakery{enter}mail{enter}t{ctrl+k}{esc}",
        flex={"desk:notes:bakery": lambda t: has_line(t, OMAR_LINE)},
        right=[("desk:notes:bakery", OMAR_LINE)],
        wrong=[("desk:notes:bakery", "closed on 2026-11-13"), ("desk:notes:bakery", "Bakery closed")],
    ),
}

PRACTICE_V2 = {
    "p-bread-reply": task(
        "practice",
        "Omar asked you something (see the newest mail). Reply to him; start your reply with the word Yes.",
        world(inbox(2, "omar", "Bread", "Hi Sam, can you pick up the bread on Friday? Omar")),
        lambda f: mail_to("omar", lambda b, m: starts_with_yes(b))(f.get(S1)),
        "one reply to omar starting with Yes",
        "mail{enter}{down}{enter}rYes, no problem!{ctrl+k}{esc}",
        flex={S1: mail_to("omar", lambda b, m: starts_with_yes(b))},
    ),
    "p-add-kai": task(
        "practice",
        "Add the person who sent the newest mail to your contacts, with their name, their mail "
        "address and their phone number (leave the note empty).",
        world(inbox(2, "kai", "Hi", "Hello Sam, this is Kai Moreno from the choir. Call me on 555-0808. Kai")),
        lambda f: norm(f.get("desk:contacts:kai")) == ["name: Kai Moreno", "mail: kai", "phone: 555-0808", "note:"],
        "contact kai: Kai Moreno, 555-0808, empty note",
        "contacts{enter}aKai Moreno{down}kai{down}555-0808{ctrl+k}{esc}",
    ),
    "p-move-dentist": task(
        "practice",
        "The dentist moved your appointment (see the newest mail). Change it in the calendar.",
        world(inbox(2, "clinic", "Appointment", "Dear Sam, your dentist appointment is moved to 2026-11-11 at 09:30.")),
        lambda f: event(f, "2026-11-11", "09:30", "Dentist") and not event(f, "2026-11-10", "09:00", "Dentist"),
        "Dentist moved to 2026-11-11 09:30",
        "calendar{enter}{enter}{bksp*10}2026-11-11{down}{bksp*5}09:30{ctrl+k}{esc}",
    ),
    "p-costs": task(
        "practice",
        "Mail Ana the total of the costs on the page costs in the lists drawer, with the subject Costs.",
        world("#page lists:costs\ncat food 12.50\nlitter 7.25\n#end\n"),
        lambda f: mail_to("ana", lambda b, m: has_amount(b, 19.75), subject="Costs")(f.get(S1)),
        "one mail to ana, subject Costs, containing 19.75",
        "mail{enter}wana{tab}Costs{tab}The total is 19.75{ctrl+k}{esc}",
        flex={S1: mail_to("ana", lambda b, m: has_amount(b, 19.75), subject="Costs")},
    ),
    "p-purple": task(
        "practice",
        "Change the theme to purple and the clock to 12h.",
        world(),
        lambda f: norm(f.get("desk:system:settings")) == ["name: friend", "theme: purple", "start: shell", "clock: 12h"],
        "settings: theme purple, clock 12h",
        "settings{enter}{down}{left}{down}{down}{right}{ctrl+k}{esc}",
    ),
}
