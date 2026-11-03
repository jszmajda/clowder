# Changelog

What changed in each release of Clowder, written so it spoils nothing: what changed for the player, never what the new things are called, what they cost, or what they do in numbers. Clowder is meant to be discovered by playing it.

## [Unreleased]

### New

- **The sidebar shows what the row under the cursor does to your stores.** A small mark beside a store says whether the row would raise it, lower it, or both, so there's no more matching names by eye. The help screen explains the marks.
- **The header says how much room your clowder has.** Beside the kitten count, it now shows how many kittens your homes can hold and how many places are free, so there's nothing to add up. It starts once you have a home, and in a very narrow pane the free places step aside first.
- **Stores say what they are.** With the cursor on a store, the info pane now opens with a little drawing in that store's colours and a line about what it is. In a pane too short to hold everything, these step aside so the figures stay.

### Changed

- **A long sleep with the game open tells its tale.** Closing your laptop with Clowder running now wakes it as a restart would: the waking screen if it was a long one, the report of what the time brought, and you left just where you were. The time also comes back in moments, where a very long sleep could take hours to work through.
- **Saves have moved up a version.** A save written by this version won't open in an older one. Older saves still open here.
- **Drawing the sidebar costs less.** It does in one step what it used to repeat for every store, which matters most in a big clowder.
- **Clowder sits lighter in its pane.** With nothing pressed, it now works out the screen about as often as something on it can change, rather than several times more. Keys answer as quickly as before; at faster speeds, figures move in slightly larger steps.
- **Redrawing the screen costs less.** Each redraw works out what each tab lists once, rather than again for every part of the screen that asks.

### Fixed

- **Coming back to a big clowder takes far less waiting.** Waking up after a long time away from a meadow that has grown large now takes a fraction of the time it did, and it wakes to exactly what it would have before.
- **Four descriptions now say what they actually do, in numbers.** One also mentions what it gives you for free.
- **Kittens say their own things.** The little moments a kitten has were showing as a single placeholder word. They now appear in the ticker and the Journal as written.
- **The Journal tells flavour from fact by colour too.** Quiet moments recede there as they do on the ticker, and stars wear their colour on tabs and event lines.
- **A few more event lines name your stores in their colours.**
- **Everything you research now does what it says.** A few things you could learn didn't open what they promised, and some of what they promised opened without them. Anything you could already reach stays yours: the research that now opens it is granted free when you load your save.
- **Looking for new faces finds the ones you've earned first.** When you've met the conditions for someone special, they now come ahead of the usual crowd.
- **An action no longer shows before it can be used.** One was listed early, and did nothing when chosen.
- **A mistyped number no longer costs you a hand-edited save.** A negative, a fraction, or a number too big for its place now reads as the nearest it can be, where it used to set the whole save aside. Something that isn't a number at all still does.
- **Hand-edited saves are harder to hurt.** A few unusual edits could empty your stores on load or be misread. They no longer do.
- **A damaged save is kept safe in one more case.** A single bad byte could slip through unnoticed and be saved over. Now that file is moved aside, you're told where it went, and a new meadow starts.
- **If clowder can't find your system's data folder, it tells you where your save is.**
- **Walking away counts right up to the moment you're back.** Quitting or changing speed just after your computer wakes no longer loses or misprices the time it slept, the first and last moments of waking up now count too, and a gap too long to count now says so.
- **Two sessions can't end up on one save in a few more ways.** Crossing a time zone no longer makes a running game look closed, a game you've taken over from no longer frees the save when it quits, and a lock that can't be cleared now says so rather than leaving the launch hanging.
- **One of the last things you can build now does all it was meant to.** Part of what it was designed to do had never been hooked up, and its description now says that part too.
- **A structure that was missed now takes the same bonuses as its neighbours.** It sat outside a production channel the others in its family all use.
- **The header's season always matches the rest of the screen.** At wide widths its word could disagree with the colours beside it.
- **The info pane no longer repeats a confirmation.** At wide widths, a summary the list already shows was drawn a second time beside it.
- **A store named in one more sentence wears its colours.**
- **A long list says what's below it.** With the cursor partway down, the note saying more rows lie below no longer falls off the pane.
- **Adventures say what they are, and which one you chose.** The list marks the adventure you've picked for the next meadow and the one under way in this one, and the info pane now tells you what each adventure asks of you and what it gives.
- **Choosing no adventure means none.** An adventure you didn't finish no longer follows you into the next meadow unless you choose it again.
- **Adventure goals can't finish themselves.** Goals that ask you to move on are now checked when you move, so none can complete before you've done anything.
- **Every adventure gives what it promises.** Finishing each one now brings what its description says, including one that used to bring nothing.
- **The adventure under way is always shown.** If the adventure list closes after a move, the adventure you're on keeps its row, so you can still see what's in effect.
- **Every structure says what it does.** The info pane now covers what a structure does beyond its table of numbers, including one that used to say nothing at all, and it says so when a figure changes with the state of your meadow.
- **A full store refuses instead of taking your payment.** One action used to spend its price even with no room left for what it makes. It now says the store is full and costs nothing.
- **A shortfall names what's actually short.** One action blamed the wrong thing when you couldn't afford it.
- **A timed pause lasts its full length.** One ended a moment early.
- **Descriptions match what happens.** A few said more, less, or something different from what the game does; they've been corrected.
- **A structure's missing bonus now works.** It was designed in but never took effect.

## [1.3.2] — 2026-10-27

### New

- **Mess can be tidied as the meadow grows.** Mess no longer has to be waited out: there are ways to clear it both when it first turns up and once industry really gets going.

### Changed

- **New achievements stand out from their names.** In the scrapbook, `new` now wears the same orange as the top-bar chip that sent you there, so it reads apart from the achievement it marks.

### Fixed

- **The mess readout stops at zero.** With nothing left to tidy, it now says mess is holding steady instead of still falling.

## [1.3.1] — 2026-10-26

### Changed

- **The achievements scrapbook is easier to read.** What you've earned stands out in colour, the names still to find read plainly with their hints quieter beside them, and a little space parts the earned from the rest.

## [1.3.0] — 2026-10-26

### New

- **Achievements.** Press `a` for the clowder's scrapbook: dozens of things worth remembering, from big milestones to small, silly kitten moments. Most show a playful hint while you're still on your way to them; some stay secret until you stumble onto them. They're just for fun and change nothing in play. The ticker celebrates each one as it's earned, and a chip in the top bar tells you when there's something new to look at, including anything earned while you were away.

### Saves

- The save format moves to version 10. An older save upgrades with nothing earned yet, and whatever your meadow has already done is credited as it plays on. A save written by this version can't be opened by an older one.

## [1.2.1] — 2026-10-24

### Fixed

- **Pen pals show what they do.** A neighbour's pen pal row now states the next letter's price, and its inspector says how many letters you've sent, how warm the neighbour is towards you and what that does to your caravans, what another letter would add, and when a closer friendship opens something more. The ticker names the neighbour and the level when a letter goes out.

## [1.2.0] — 2026-10-24

### New

- **Every kitten has a profile.** Press `Space` on a kitten in the Clowder section (`2`) to see its card: a small portrait in its own fur, pose, and eye colour; a strip naming its colours; a few facts about it that matter for play; and a handful of favourites and quirks, different kinds for every kitten. Once in a blue moon a kitten arrives looking like no ordinary cat.
- **Energy and mess at a glance.** The Meadow (`1`) shows the energy you make against the energy you use, and how messy things are. `Space` on either explains where it comes from and what it is slowing down, and a warning sits in the top bar whenever energy runs short.
- **Every purchasable item says what it does.** Sunbeam unlocks and the other kinds that used to show nothing now describe themselves in the inspector.
- **Where to look for a store you're short of.** When a price asks for something nothing in your meadow is making, the inspector says where it can come from, and hints at places you haven't found yet.
- **What your neighbours have shown you.** Each neighbour's trade row lists the goods its caravans have brought home, and hints at what else it has.
- **A summary heads the crafting list,** saying who is crafting what and who is waiting their turn.
- **What runs on a clock.** The ticker shows what is running and how long it has left, and tells you what a pounce or a trade brought home.
- **What moves each store.** A store's inspector lists everything filling it and everything drawing it down.
- **Resource names wear colours of their own,** wherever they appear.
- **The stores sidebar scrolls,** with `{` and `}` or the mouse wheel.
- **A grown clowder has enough names to go round.**

### Changed

- A building appears the moment its research opens it.
- The kittens list sits last on its screen, so the things to do stay in reach.
- Sky events stay open to observe for longer.

### Fixed

- **Shinies** have a steady supply, and pouncing brings home a fair share of them.
- **Energy** no longer leaves a meadow short for a whole stretch of the game, waiting on producers that arrive later.
- **Economy gaps an audit found:** a material that couldn't be made without already having some, two crafts nothing ever used, a luxury with too thin a supply, and a neighbour whose terms made trading with it pointless.
- **Five purchasable items that did nothing when bought** now work.

### Saves

- The save format moves to version 9. Older saves upgrade when loaded: every kitten gains a profile, and nothing else in the meadow changes. A save written by 1.2 can't be opened by an older version.

## [1.1.0] — 2026-10-14

### New

- **The mouse moves around.** Click a tab or a row to move there, and scroll lists with the wheel. A click never acts, so clicking into the pane can't spend anything by accident.
- **Finished research folds away** into a group of its own at the foot of the list.
- **The journal scrolls from the keyboard** as well as with the wheel.

## [1.0.0] — 2026-10-14

The first release.
