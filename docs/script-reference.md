# Unseen Script reference (M1 to M5: values, blocks, tasks, library, pages, try, tests, the OS, apps)

This reference grows with each milestone. **Every example here is a test**: the
`script` block is run, and it must give exactly the `output` block below it (or, for a
`mistake` block, a mistake that contains that text). A `tests` block is run as a test page and
must give exactly its `report`; a `page` block puts that page on the examples' desk. Every line of every example is also
typed through PowerShell and bash, to check the computer receives exactly that text.

The examples run on a small test desk: two contacts (`contacts:omar`, `contacts:lina`), three
mails in `mail:inbox` (`m1`, `m2`, `m10`), the page `notes:shopping` (eggs, bread, tea), and
today is 2026-11-12.

## Lines

One instruction per line. Spaces at the start of a line don't matter. `--` starts a comment.
Capitals don't matter in words and names. Every chore is also an Unseen Script program.

```script
set Total to 2 -- a comment
say total * 3
```
```output
6
```

## Values

| Kind | Written like |
|---|---|
| number | `12`, `3.5`, `-2` |
| text | `'hello'` or `"hello"`; a doubled quote mark inside is one quote mark: `'it''s'` |
| yes / no | `yes`, `no` |
| list | `[1, 'a', yes]`, `[]` (the first item is 1) |
| record | `{name: 'Mira', age: 3}`, `{}` |
| nothing | `nothing()` |

```script
say 'it''s ' & 3.5 & ' ' & yes
say [1, 'a', [2, 3]]
say {name: 'Mira', age: 3}
say nothing()
```
```output
it's 3.5 yes
1, a, 2, 3
{name: Mira, age: 3}
nothing
```

## Lists

`count(list)` is the number of items, `item(list, n)` is item n. `add VALUE to NAME` puts an
item at the end. `set NAME at N to VALUE` changes item N.

```script
set shop to ['eggs', 'tea']
add 'milk' to shop
set shop at 1 to 'bread'
say count(shop) & ': ' & shop
say item(shop, 3)
say shop has 'tea'
```
```output
3: bread, tea, milk
milk
yes
```

A list can hold anything, even other lists. `has` looks for an item of the same kind.

```script
say [1, 2, 3] has 2
say [1, 2, 3] has '2'
say [[1, 2], [3]] has [3]
```
```output
yes
no
yes
```

```script
set l to [1, 2]
set l at 3 to 9
```
```mistake
item 3 doesn't exist: the list has 2 items
```

## Records

A record has named fields. `r.name` reads one; `set r.name to VALUE` changes it, or makes it
if it is new. Field names follow the rules of names.

```script
set pet to {name: 'Mira', age: 3}
set pet.age to pet.age + 1
set pet.city to 'Gent'
say pet.name & ' is ' & pet.age
say pet
say count(pet)
```
```output
Mira is 4
{name: Mira, age: 4, city: Gent}
3
```

Records can hold records and lists. To change something deeper, give the whole path. Every
part before the last must already exist.

```script
set s to {who: {name: 'Omar'}, items: []}
set s.who.name to 'Lina'
add 'tea' to s.items
say s.who.name & ' wants ' & s.items
```
```output
Lina wants tea
```

```script
set pet to {name: 'Mira'}
say pet.age
```
```mistake
the record has no field age (it has: name)
```

## Values are copies

`set b to a` makes a copy: changing `b` never changes `a`, however deep the change.

```script
set a to {items: [1]}
set b to a
add 2 to b.items
say a.items
say b.items
```
```output
1
1, 2
```

## Comparing

`is` and `is not` compare two values of the same kind. Lists are the same if they have the
same items in the same order; records if they have the same fields with the same values (the
order of the fields doesn't count). Anything can be compared with `nothing()`.

```script
say [1, 2] is [1, 2]
say {a: 1, b: 2} is {b: 2, a: 1}
say 5 is nothing()
```
```output
yes
yes
no
```

```script
say 1 is '1'
```
```mistake
is compares two values of the same kind
```

## when, or when, otherwise

```script
set n to 7
when n < 0
  say 'below zero'
or when n < 10
  say 'small'
otherwise
  say 'big'
done
```
```output
small
```

## Loops

`for each NAME in LIST`, `repeat NUMBER times` and `while CONDITION` repeat the lines up to
`done`. `skip` goes on with the next turn; `quit` ends the loop.

```script
set n to 0
while n < 3
  set n to n + 1
done
for each x in [1, 2, 3, 4, 5]
  when x is 2
    skip
  done
  when x is 4
    quit
  done
  say x
done
say 'n is ' & n
```
```output
1
3
n is 3
```

## Reading pages

Everything chores can read works the same: `pages`, `read`, `field`, `body`, `exists`,
`lines`, `words`, `count`, `item`, `number`, `lower`, `today`, `weekday`, `add_days`.

```script
set from to []
for each m in pages('mail:inbox')
  add field(m, 'from') to from
done
say from
say count(lines(read('notes:shopping')))
```
```output
omar, lina, omar
3
```

## Changing pages

`write`, `add`, `toss`, `send` and `event` work as in chores. They are planned in order, and
made together at the end, only if the program finished without a mistake. Reading still sees
the pages as they were at the start.

```script
write('desk:list', 'tea')
add('desk:list', 'milk')
say exists('desk:list')
```
```output
no
```

```script
toss('notes:shopping')
toss('notes:shopping')
```
```mistake
line 2: there is no page called desk:notes:shopping
```

## Tasks

A **task** is a piece of program with a name. `task NAME(INPUT, ...)` up to `done` makes one;
`give VALUE` ends it and gives the value back. Call it like a built-in function. A task can be
used before the line that makes it, and tasks can call each other and themselves.

```script
task tax(amount)
  give amount * 21 / 100
done
task with_tax(amount)
  give amount + tax(amount)
done
say with_tax(200)
```
```output
242
```

A task sees only its inputs and its own names (and the fixed values). Inputs are copies:
changing one inside the task never changes the caller's value. A task that ends without
`give` gives `nothing()`; `give` alone does that too.

```script
task grow(list)
  add 'more' to list
  give list
done
set mine to ['one']
set bigger to grow(mine)
say mine
say bigger
```
```output
one
one, more
```

A task called on a line of its own throws away what it gives. Its changes to pages are planned
like any other.

```script
task note(text)
  add('desk:log', text)
done
note('first')
note('second')
say 'two lines planned'
```
```output
two lines planned
```

`give` from inside a loop ends the task at once:

```script
task first_long(words)
  for each w in words
    when count(w) > 3
      give w
    done
  done
  give ''
done
say first_long(['to', 'tea', 'bread', 'milk'])
```
```output
bread
```

Tasks can call themselves (at most 40 calls deep):

```script
task fact(n)
  when n <= 1
    give 1
  done
  give n * fact(n - 1)
done
say fact(6)
```
```output
720
```

A mistake inside a task names its line, and the lines it was called from:

```script
task half(n)
  give n / 0
done
say half(4)
```
```mistake
line 2: dividing by 0 is not possible
  called from desk:main line 4
```

Calls are checked before anything runs: a task that doesn't exist, or the wrong number of
inputs, stops the program before its first line.

```script
say 'this never shows'
say tax(1, 2)
task tax(amount)
  give amount / 5
done
```
```mistake
line 2: tax(...) needs 1 value in its brackets, got 2
```

## Fixed values

`fixed NAME is VALUE` makes a value that never changes, and that every task can see. It may
use numbers, texts, lists, records, operators, earlier fixed values and built-in functions that
don't look at the pages. Fixed values are worked out before the program runs.

```script
fixed rate is 21
fixed shops is ['bakery', 'market']
task tax(amount)
  give amount * rate / 100
done
say tax(100) & ' at the ' & first(shops)
```
```output
21 at the bakery
```

```script
fixed rate is 21
set rate to 6
```
```mistake
rate is a fixed value: it can't be changed
```

## Text functions

| Function | Gives |
|---|---|
| `upper(t)`, `lower(t)` | `t` in capitals, in small letters |
| `trim(t)` | `t` without spaces at the start and end |
| `split(t, by)` | list of the parts of `t` between each `by` |
| `join(list, by)` | the items as one text, with `by` between them |
| `replace(t, old, new)` | `t` with every `old` changed to `new` |
| `find(t, part)` | where `part` starts in `t` (1 is the first letter), 0 if it isn't there |
| `slice(t, from, to)` | letters `from` to `to` (both included; also works on lists) |
| `starts(t, part)`, `ends(t, part)` | yes if `t` starts / ends with `part` |
| `newline()` | a line break |
| `text(value)` | any value as text |
| `pad(t, width)` | `t` with spaces after it up to `width` letters (before it, if `width` is below 0) |

Positions and lengths count the letters, digits and signs the keyboard types: one place each.
(A character the keyboard can't type, like `é`, takes more than one place; `slice` never cuts
one in half. `upper` and `lower` only change a to z.)

```script
set line to '  Milk, eggs ,bread '
set items to split(trim(line), ',')
say count(items) & ' items'
set clean to []
for each i in items
  add trim(i) to clean
done
say join(clean, ' + ')
say upper(slice('bread', 1, 1)) & slice('bread', 2, 5)
say replace('a-b-c', '-', '/') & ' ' & find('hello', 'l')
say '[' & pad('tea', 6) & '][' & pad('42', -5) & ']'
```
```output
3 items
Milk + eggs + bread
Bread
a/b/c 3
[tea   ][   42]
```

## Number functions

`round(n, decimals)` (0 to 6 decimals), `floor(n)`, `ceil(n)`, `abs(n)`, `min(a, b)` and
`max(a, b)` (two numbers or two texts), `is_number(t)` (yes if `number(t)` would work).

```script
say round(2.346, 2) & ' ' & floor(2.9) & ' ' & ceil(2.1) & ' ' & abs(-4)
say min(3, 8) & ' ' & max('apple', 'pear')
say is_number('12.5') & ' ' & is_number('12,5')
```
```output
2.35 2 3 4
3 pear
yes no
```

## List functions

| Function | Gives |
|---|---|
| `sum(list)` | the numbers added up (0 for `[]`) |
| `sort(list)` | the items in order (all numbers, or all texts) |
| `reverse(list)` | the items the other way round |
| `unique(list)` | the list without repeats (the first of each is kept) |
| `first(list)`, `last(list)` | the first / last item (a mistake if the list is empty) |
| `range(from, to)` | the whole numbers from `from` to `to` |
| `position(list, value)` | where `value` is in the list, 0 if it isn't there |
| `without(list, n)` | the list without item `n` |

```script
set prices to [4, 1.5, 3, 1.5]
say sum(prices)
say sort(prices)
say unique(prices)
say first(sort(prices)) & ' to ' & last(sort(prices))
say range(1, 5)
say position(['tea', 'milk'], 'milk') & ' ' & without(['a', 'b', 'c'], 2)
```
```output
10
1.5, 1.5, 3, 4
4, 1.5, 3
1.5 to 4
1, 2, 3, 4, 5
2 a, c
```

## Record functions

`keys(record)` is the list of its field names; `has_key(record, name)` says whether it has that
field. `fields(page)` reads every `key: value` line at the top of a page into a record.

```script
set m to fields('mail:inbox:m2')
say keys(m)
say m.from & ' wrote about ' & m.subject
say has_key(m, 'subject') & ' ' & has_key(m, 'date')
```
```output
from, to, subject
lina wrote about Lesson
yes no
```

## Date functions

`days_between(a, b)` is the number of days from `a` to `b`; `month(date)` and `year(date)` are
numbers. (Also from chores: `today()`, `weekday(date)`, `add_days(date, n)`.)

```script
say days_between(today(), '2026-12-25') & ' days to go'
say month('2026-11-12') & '/' & year('2026-11-12')
```
```output
43 days to go
11/2026
```

## Pages that use pages

A program can be spread over several pages. `use 'PAGE'` at the top of a page loads the tasks
and fixed values of another page. A page that is used may only hold `use`, `fixed` and `task`
lines, so loading it never does anything by itself. Pages can use pages that use pages; each
page is loaded once, and two pages may use the same page. Paths always start at the desk.

These two pages are on the examples' desk:

```page lib:money
-- money tools
fixed rate is 21

task tax(amount)
  give amount * rate / 100
done
```

```page lib:shop
use 'desk:lib:money'

task price(amount)
  give 'EUR ' & (amount + tax(amount))
done
```

```script
use 'desk:lib:shop'
say price(100)
say 'tax is ' & rate & ' percent'
```
```output
EUR 121
tax is 21 percent
```

A mistake on a used page names that page, and the line it was called from:

```page lib:broken
task half(n)
  give n / 0
done
```

```script
use 'lib:broken'
say half(10)
```
```mistake
desk:lib:broken line 2: dividing by 0 is not possible
  called from desk:main line 2
```

`use` must come at the top, before anything else:

```script
say 'hello'
use 'lib:money'
```
```mistake
use must come at the top of the page
```

## fail, try and instead

`fail VALUE` stops the program with your own mistake. `try` ... `instead NAME` ... `done`
catches mistakes: if a mistake happens in the `try` part (also inside tasks it calls),
**everything the try part did is undone**: its names, its `say` lines and its planned
changes. Then the `instead` part runs, with the mistake's message as text in `NAME`.

```script
task check_amount(n)
  when n < 0
    fail 'an amount can''t be below 0, got ' & n
  done
  give n
done
set total to 0
for each n in [5, -2, 7]
  try
    set total to total + check_amount(n)
    say 'added ' & n
  instead problem
    say 'skipped: ' & problem
  done
done
say 'total: ' & total
```
```output
added 5
skipped: an amount can't be below 0, got -2
added 7
total: 12
```

```script
set x to 1
try
  set x to 2
  write('desk:list', 'tea')
  say 'this is undone'
  say 10 / 0
instead problem
  say 'x is still ' & x
done
```
```output
x is still 1
```

Going over a limit (too many steps, too much memory, too deep) and `stop` can't be caught.

## Test pages

A **test page** holds `use` lines and `test 'TITLE'` ... `done` blocks. The `test` command
runs every test on it and shows `PASS` or `FAIL` for each.

- `check CONDITION` fails the test if CONDITION is no. For `check a is b` a failed check shows
  both values.
- A test passes only if it reaches its `done` with at least one check, and every check is yes.
  A mistake, `stop`, or no checks at all is a FAIL.
- **Each test starts fresh**, in its own **test world**: the empty app drawers (`mail:inbox`,
  `mail:sent`, `contacts`, `calendar`), the default settings, and today is 2026-11-14. The
  code comes from the real desk; the pages the program reads come from the test world.
- At the top of a test, `pretend 'PATH' holds TEXT` puts a page in the test world (with the
  drawers above it), and `pretend today is 'DATE'` sets the date.
- **Nothing a test does is ever made.** `after(PATH)` is the text a page would have after the
  changes the test planned (`nothing()` if it would not exist), `page_now(PATH)` its text
  without them, `sent()` the mails it would send (records with `to`, `subject`, `text`),
  `events()` the events it would add (`date`, `time`, `title`, `with`), and `said()` its
  `say` lines. These only work in tests.

```page lib:list
fixed list_page is 'desk:shop:list'

task load_items()
  when exists(list_page)
    give lines(read(list_page))
  done
  give []
done

task save_items(items)
  write(list_page, join(items, newline()))
done
```

```tests
use 'desk:lib:list'

test 'nothing saved yet'
  check load_items() is []
done

test 'it reads the saved list'
  pretend 'desk:shop:list' holds 'milk' & newline() & 'eggs'
  check load_items() is ['milk', 'eggs']
done

test 'saving plans a write'
  pretend 'desk:shop:list' holds 'milk'
  save_items(['milk', 'tea'])
  check page_now('desk:shop:list') is 'milk'
  check after('desk:shop:list') is 'milk' & newline() & 'tea'
done

test 'a wrong expectation'
  check count(load_items()) is 2
done
```
```report
PASS nothing saved yet
PASS it reads the saved list
PASS saving plans a write
FAIL a wrong expectation: line 20: check failed: got 0, expected 2
3 of 4 tests passed
```

```tests
test 'a mail is planned, not sent'
  pretend 'contacts:mira' holds 'name: Mira'
  send('mira', 'Party', 'Come at 8')
  say 'invited'
  check first(sent()).to is 'mira'
  check said() is ['invited']
  check count(pages('mail:sent')) is 0
done

test 'the date can be pretended'
  pretend today is '2027-01-01'
  check weekday(today()) is 'friday'
done
```
```report
PASS a mail is planned, not sent
PASS the date can be pretended
2 of 2 tests passed
```

## Programs in the OS

A program is a page. These words work at the prompt (`guide` lists them under Programs):

| Command | What it does |
|---|---|
| `run PAGE` | run the program on PAGE; an Output screen shows what it said and the changes it made |
| `test PAGE` | run the tests on a test page; the screen shows `PASS` or `FAIL` for each |
| `lab` | the try-out screen (below) |
| `view PAGE [FROM] [TO]` | show the lines of PAGE with their numbers (at most 20 at a time) |
| `edit PAGE LINE TEXT` | replace line LINE with TEXT |
| `insert PAGE LINE TEXT` | put TEXT in as a new line before line LINE; LINE = last + 1 adds it at the end, and `insert PAGE 1 TEXT` makes a page that isn't there yet |
| `cut PAGE LINE [TO]` | remove line LINE, or the lines LINE to TO |

For `edit` and `insert`, the text starts after exactly one space, so spaces at its start are
kept (lines keep their indent), and no text at all means an empty line. A page's lines are
its text split at each new line, as in Notes: an empty page is one empty line. A line is at most 78
characters, the width of Notes. Notes shows line numbers too: **Ctrl+G** goes to a line and
**Ctrl+F** finds a word (Ctrl+F then Enter finds the next one).

A mistake names the page and line, so it can be fixed with one `edit`. This builds a small
program in the drawer `tools`, tests it, fixes it and runs it:

```commands
build tools
insert tools:money 1 fixed rate is 21
insert tools:money 2 task tax(amount)
insert tools:money 3   give amount * rate / 10
insert tools:money 4 done
insert tools:check 1 use 'tools:money'
insert tools:check 2 test 'tax of 100'
insert tools:check 3   check tax(100) is 21
insert tools:check 4 done
test tools:check
edit tools:money 3   give amount * rate / 100
test tools:check
insert tools:main 1 use 'tools:money'
insert tools:main 2 say 'tax on 200: ' & tax(200)
run tools:main
```

The first `test` shows `FAIL tax of 100: line 3: check failed: got 210, expected 21`; after
the `edit`, both tests pass, and `run tools:main` shows `tax on 200: 42`.

## The lab

`lab` opens a screen to try things out one line at a time. Type a line and press Enter:

- a **value** (`1 + 2`, `items`, `count(pages('mail:inbox'))`, `tax(100)`) is shown as you
  would type it: texts in quotes, lists in `[ ]`, records in `{ }`;
- an **instruction** is carried out, and shows its `say` lines. A block (`when`, `for each`,
  `task`, `try`, ...) runs when its `done` is typed; until then the lab shows `...`;
- **names, tasks, fixed values and `use` lines are kept** while the lab is open, so later lines
  can use them. A line with a mistake is not kept;
- **changes are only shown**, never made: `would write desk:list`.

Esc leaves the lab, and everything in it is forgotten. Up brings back the last line typed.
A new `fixed` or `use` line that would change what earlier lines did (an earlier `try` now
going the other way) is refused, because the lab can't show the earlier lines again.
The lab keeps the lines that worked and runs them again with every new line, so all of them
together must stay within the limits of one program (400 lines, 500000 steps).

```lab
>   set items to ['milk']
>   add 'tea' to items
>   items
['milk', 'tea']
>   count(items) * 2
4
>   {name: 'Mira', likes: items}
{name: 'Mira', likes: ['milk', 'tea']}
>   'it''s'
'it''s'
>   write('desk:list', join(items, ', '))
would write desk:list
>   for each i in items
...   say 'buy ' & i
... done
buy milk
buy tea
>   use 'lib:money'
>   tax(200)
42
>   items / 2
mistake: / needs two numbers, got a list of 2 items
```

## Apps

An **app** is a page that starts with `app 'TITLE'` and has three tasks:

- `start()` gives the app's first **state** (any value, usually a record);
- `key(s, k)` gets the state and a key, and gives the new state;
- `draw(s)` shows the state on the screen with the screen functions below.

`open PAGE` checks the app and runs `start`, then `draw`. Every key is one more **step**:
`key`, then `draw` with the state it gave. A step is all or nothing: when it ends, its changes
to pages are made, its state is kept and its screen is shown. If anything in a step goes
wrong, none of that happens: the app closes and the Output screen shows the mistake (Esc goes
back). **Ctrl+Q** always closes the app, and is never given to `key`.

```page apps:shop-lib
fixed list_page is 'desk:shop-list'

task load_items()
  when exists(list_page)
    give lines(read(list_page))
  done
  give []
done

task save_items(items)
  write(list_page, join(items, newline()))
done
```

```page apps:shop
app 'Shopping list'
use 'desk:apps:shop-lib'

task start()
  give {items: load_items(), picked: 1, typing: ''}
done

task key(s, k)
  when k is 'down' and s.picked < count(s.items)
    set s.picked to s.picked + 1
  or when k is 'up' and s.picked > 1
    set s.picked to s.picked - 1
  or when k is 'enter' and s.typing is not ''
    add s.typing to s.items
    set s.typing to ''
    save_items(s.items)
  or when k is 'delete' and count(s.items) > 0
    set s.items to without(s.items, s.picked)
    set s.picked to max(min(s.picked, count(s.items)), 1)
    save_items(s.items)
  otherwise
    set s.typing to edit_text(s.typing, k)
  done
  give s
done

task draw(s)
  title('Shopping list - ' & count(s.items) & ' items')
  for each n in range(1, min(count(s.items), 20))
    when n is s.picked
      show(n, 2, '> ' & item(s.items, n), 'bright')
    otherwise
      show(n, 2, '  ' & item(s.items, n))
    done
  done
  show(22, 2, 'new: ' & s.typing)
  help('Up/Down pick  Enter add  Delete remove')
done
```

`start` and `key` must end with `give`. `close()` (in `start` or `key`) closes the app at the
end of the step: its changes are made, and `draw` doesn't run. The state kept between steps
is at most 256 KB.

**Keys** reach `key` as text: one character (`'a'`, `'A'`, `'7'`, `' '`, `'.'`), or `'enter'`,
`'esc'`, `'backspace'`, `'tab'`, `'delete'`, `'up'`, `'down'`, `'left'`, `'right'`, `'home'`,
`'end'`, and `'ctrl+a'` to `'ctrl+z'`. `edit_text(text, k)` types a key into a text: a
character is added, `'backspace'` takes the last one away, other keys change nothing.

**The screen** has rows 1 to 23 and columns 1 to 78; the top bar shows the title, the bottom
bar the help. It is empty at the start of every `draw`.

| Function | What it does |
|---|---|
| `show(ROW, COL, TEXT)` | put TEXT there (any value is shown as text; it is cut at column 78) |
| `show(ROW, COL, TEXT, COLOUR)` | the same, in a colour: `'normal'`, `'bright'`, `'accent'`, `'warn'` or `'select'` |
| `fill(ROW, COLOUR)` | colour a whole row (a `show` without a colour keeps it) |
| `title(TEXT)` | the top bar (otherwise the app's TITLE) |
| `help(TEXT)` | the bottom bar; `Ctrl+Q close` is added if it is not in it (the help is cut so it fits) |
| `cursor(ROW, COL)` | show the blinking cursor there (otherwise it is hidden) |

**What each part may do** is checked when it happens, also in helper tasks:

| Part | read pages | change pages | screen functions | `close()` | `say` |
|---|---|---|---|---|---|
| `run`, and a test's own lines | yes | yes | no | no | yes |
| `start`, `key` | yes | yes | no | yes | no |
| `draw` | no | no | yes | no | no |

So `draw` works only from the state, and `stop` in an app is a mistake (use `close()`).

```script
show(1, 1, 'hello')
```
```mistake
show(...) only works in an app's draw
```

```script
use 'desk:apps:shop'
say start().picked
```
```output
1
```

### Testing an app

In a test, `launch 'PAGE'` opens an app exactly as `open` does, and `press 'KEY'` is one key
step. Each step is real: its changes are made **in the test world**, so the next step sees
them (and nothing is ever made on the desk). `press 'ctrl+q'` closes the app. After a step, a
test can look at:

- `app_state()`: the app's state;
- `screen_line(ROW)`: a row as text (0 is the title bar, 24 the help bar);
- `screen_colour(ROW, COL)`: the colour at that place;
- `app_open()`: yes while the app is open; `app_mistake()`: the mistake that closed it (`''`
  if none). A failing step closes the app as in the OS; the test goes on.

`page_now`, `sent()` and `events()` include what the app's steps made. Pressing a key after
the app has closed fails the test. A step's changes can't be undone, so `launch` and `press`
can't be in the try part of a `try`. The test's own lines and all its app's steps (also the
ones that fail) may take at most 5000000 steps together; the test world holds at most 1 MB and
5000 drawers and pages, like the desk.

```tests
use 'desk:apps:shop'

test 'enter adds the typed item and saves it'
  pretend 'desk:shop-list' holds 'milk'
  launch 'desk:apps:shop'
  press 'b'
  press 'enter'
  check app_state().items is ['milk', 'b']
  check page_now('desk:shop-list') is 'milk' & newline() & 'b'
  check screen_line(2) has 'b'
done

test 'the picked item is shown bright'
  pretend 'desk:shop-list' holds 'milk' & newline() & 'eggs'
  launch 'desk:apps:shop'
  press 'down'
  check screen_line(2) has '> eggs'
  check screen_colour(2, 2) is 'bright'
  check screen_line(24) is 'Up/Down pick  Enter add  Delete remove   Ctrl+Q close'
done

test 'up and down on an empty list change nothing'
  launch 'desk:apps:shop'
  press 'down'
  press 'up'
  check app_state().picked is 1
  press 'x'
  press 'enter'
  check app_state().items is ['x']
done

test 'ctrl+q closes it'
  launch 'desk:apps:shop'
  press 'ctrl+q'
  check not app_open()
  check app_mistake() is ''
done
```
```report
PASS enter adds the typed item and saves it
PASS the picked item is shown bright
PASS up and down on an empty list change nothing
PASS ctrl+q closes it
4 of 4 tests passed
```

At the prompt:

```commands
open apps:shop
```

## Limits

Going over a limit is a mistake that ends the program: 400 lines and 16000 characters per
page; blocks 16 deep; brackets 12 deep in one line; 8000 characters per text; 2000 items per
list; lists and records nested 16 deep; tasks 40 calls deep; 500 `say` lines; 300 changes;
500000 steps; 1 MB of memory.

```script
set l to 1
repeat 17 times
  set l to [l]
done
```
```mistake
nested at most 16 deep
```
