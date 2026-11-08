# Website step 8: draft text (for the site thread; not yet on the site)

## Step 8: Can an AI program in a language made just for this computer?

**The question:** how much does the manual help when an AI has to program for real?

Unseen OS has its own programming language, Unseen Script. It is a custom language we made for
this project. We gave two small AI models,
Haiku and Luna, 24 real programming jobs: build a new app, add a feature, or fix a bug. Every
game started with a fresh model session. Hidden tests, which the models never saw, checked
whether each app really worked.

Each model did every job 4 times, in three ways:

- **No help:** a short language guide is on the computer, but the model has to look it up there.
- **Reference:** the same 255-line language guide is handed to the model up front.
- **Full manual:** the whole 961-line manual, which includes that guide, is handed to the model
  up front.

That made 576 scored games in total.

**What we found**

| | No help | Reference | Full manual |
|---|---|---|---|
| Haiku | 90% passed | 99% | 95% |
| Luna | 74% passed | 83% | 91% |

- **The manual helps.** The models passed 11 percentage points more often with the full manual
  than without help. A result this large would be very unlikely by chance (p = 0.0001), assuming
  the 24 jobs behave independently. Some jobs share the same base app, so that assumption is
  not perfect.
- **Fewer steps.** The jobs the models finished with the manual took 43% fewer steps than the
  jobs they finished without help.
- **Reference or full manual?** We found no clear difference between the short guide and the
  full manual. That doesn't prove they are equally good.
- **Luna gained most.** Luna improved by 17 points, Haiku by 5. Haiku was already passing 90%
  without help.

**Fair-play notes**

- We wrote down all the rules before the first game and locked the files.
- After the run, the locked analysis refused to give a verdict over a formatting detail: the
  list of conditions was stored as a list instead of as text. We fixed only that, openly
  ([deviations](deviations5.md)), and no result changed.
- Every one of the 576 scored games counts, including 22 lost for breaking the command rule.
- 14 more attempts were cut off by a planned pause or by an AI tool error. They were replayed
  under the rules we had written down in advance.
