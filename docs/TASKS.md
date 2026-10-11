# The board of tasks

Agents find work they do not do now, and people keep a list of what to do. Leon
keeps one board for both: tasks in Leon's own database, a command line to reach
them, and an MCP server so an agent can use them without a shell.

The board is in the window (`⇧⌘I`, `Ctrl+Shift+Alt+I` elsewhere), in the
command line (`leon task`) and in an MCP server (`leon task mcp`), all reading
and writing the same database. The synchronisation with GitHub issues and
Linear comes next and is not in Leon yet.

Nothing here writes into a project or into an agent's configuration. `leon task
mcp` is registered by the person who wants it.

## Tasks and boards

A task is a **title** (one line, at most 200 characters), a **body** (at most
20,000), a **status**, a **priority**, up to 10 **labels**, an **assignee** and,
when known, the **agent** that created it.

| Status | Meaning |
| --- | --- |
| `backlog` | Known, not planned |
| `todo` | Planned, not started (the default) |
| `doing` | Being worked on |
| `review` | Done, waiting to be looked at |
| `done` | Finished |
| `canceled` | Dropped without being done |

Priorities are `none`, `low`, `medium`, `high` and `urgent` (0 to 4). The words
the trackers use for the same columns are read too: `in-progress`, `started`,
`in-review`, `closed`, `cancelled`.

A task belongs to a board: the one of a **project**, which is the project the
current folder belongs to and is shared by all its git worktrees (resolved
exactly as the [shared memory](MEMORY.md#entries-and-scopes) resolves it), or
the **global** one (`--global`), which belongs to no project. A task of
another project is not found by its id from this one.

Cards are ordered inside a column by a position: a new task goes to the bottom
of its column, `move --before <id>` places it above another card.

## Not duplicating

Creating a task whose title (case, spacing and surrounding punctuation
aside) an **open** task of the board already has creates nothing and answers
with that task. An agent that retries, or two that find the same chore, do not
fill the board. A done or canceled task does not count: the same chore can come
back. A board holds at most 100,000 tasks, a guard against an agent that
creates in a loop.

## The board in the window

The board is the main pane, as the Den is, and the sidebar stays beside it. It
opens from the **Tasks** row under Home, `⇧⌘I` (`Ctrl+Shift+Alt+I` on Linux and
Windows), **Show the task board** in the palette, or `leon --tasks` at the
start; the same command, `Esc` or the row closes it and puts back what the
pane showed. **New task…** in the palette creates a task from anywhere. The
columns are `backlog`, `todo`, `doing`, `review` and `done` (a canceled task
has no column; `leon task list --all` shows it).

The header names the project and counts what is open and done, and has one
chip per project of this computer and **Global**. A project on another
machine has no board here: its tasks live on that machine.

### Dragging

Drag a card with the pointer to any place of any column: a line marks where it
would land, the column it is over lights up, and an empty column takes the
card in all its height. Letting go outside the columns, or `Esc`, drops
nothing. Dropping a card where it already is changes nothing.

### Keys

| Key | What it does |
| --- | --- |
| `←` `→` or `h` `l` | Cursor to the column on the left or right |
| `↑` `↓` or `k` `j` | Cursor to the card above or below |
| `Shift` + those | Carry the card: to the bottom of the next column, or one place up or down in its own |
| `w` or `Enter` | Start work on the card (see below), or open its worktree |
| `n` | Ask for the title of a new task, in the cursor's column |
| `d` | Finish the card |
| `p` | Raise its priority: none, low, medium, high, urgent, none |
| `g` | Switch between the project's board and the global one |
| `Esc` | Let go of a drag, or close the board |

A click selects a card; the strip under the columns shows it in full with its
buttons. Every change goes through the engine, as every write of the window
does, so the status line says what was done (`Moved 1a2b3c4d to doing: ...`).
The open board reads the store every second and a half because an agent writes
from another process; a card that moved is followed by the cursor.

### Start work on a task

`w` on a card of a project's board asks which agent codes it (the agents
installed here that take a prompt, or **Only a worktree**), then:

1. makes a worktree named from the title and the task's short id
   (`fix-the-login-1a2b3c4d`; a number is added if git has that name);
2. starts the agent in it with the task as its prompt: the title, the body and
   how to move the card to review when it is done (`"$LEON_BIN" task move <id>
   review`), behind the project's `setup` when its `leon.toml` has one;
3. **only then** writes the card: its branch, the column `doing` and, when
   nobody had it, the agent as assignee.

A worktree that git refuses leaves the card where it was and says why. A card
with a branch shows it on the card, and `w` opens that worktree instead of
making another; when the worktree is gone, `w` starts the work again. The
global board cannot start work: it has no project to make a worktree in.

## The command line

```text
leon task add [--global] [--status S] [--priority P] [--label L]... [--assignee A]
              [--agent A] [--body B | --body -] [--json] <title...>
leon task list [--global] [--status S] [--all] [--assignee A] [--label L] [--limit N] [--json]
leon task board [--global]
leon task show <id> [--json]
leon task edit <id> [--title T] [--body B | --body -] [--status S] [--priority P]
               [--label L]... | --no-labels] [--assignee A | --no-assignee]
               [--branch B | --no-branch]
leon task move <id> <status> [--before <id>]
leon task done <id>
leon task rm <id>
leon task search [--global] [--limit N] [--json] <words...>
leon task mcp
```

| Command | What it does |
| --- | --- |
| `add` | Creates a task. The title is the remaining words. `--body -` reads the body from standard input. `--agent` names the creator; without it Claude Code, opencode and Gemini are recognised from the environment they give their commands. |
| `list` | The board's tasks in the order of the board. Done and canceled ones are left out unless `--all` or `--status` asks for them. |
| `board` | The columns, each with its tasks. |
| `show` | One task in full (`--json`: every field). |
| `edit` | Changes what is named; what is not stays. `--label` replaces the labels, `--no-labels` clears them. |
| `move`, `done` | Moves a task to a column, to the bottom of it or above `--before`. `done` is `move <id> done`. |
| `rm` | Removes a task for good. |
| `search` | Full-text search of titles and bodies, of the project's board and the global one together. |
| `mcp` | Serves the board over MCP on standard input and output. |

An id is the one printed in the first column of a listing, or the end of an id
of at least six characters. `--json` prints the tasks as JSON with every field,
the times in milliseconds since the Unix epoch. The data folder is `--data-dir`
(before the word `task`), else the `LEON_DATA_DIR` a terminal of Leon was given,
else the platform's.

## The MCP server

`leon task mcp` speaks the Model Context Protocol over standard input and
output. Register it in the agent's own configuration, for example
`claude mcp add leon-tasks -- leon task mcp`. The project is the folder the
agent was started in (`CLAUDE_PROJECT_DIR` when the client sets it).

| Tool | What it does |
| --- | --- |
| `task_create` | Adds a task, or answers the open one with the same title (`created: false`). |
| `task_list` | The open tasks in the order of the board; `include_closed`, `status`, `assignee`, `label`, `scope`, `limit`. |
| `task_get` | One task in full. |
| `task_update` | Changes title, body, status, priority, labels, assignee or the branch of its worktree. |
| `task_move` | Moves a task to a column, to the bottom or above another task. |
| `task_search` | Searches titles and bodies of this project's board and the global one. |

No tool deletes a task: an agent that no longer wants one cancels it, and
removing for good is `leon task rm`, the person's.

## Where it is stored

In `leon.db`, table `task` (migrations 20 and 21) with a full-text index over the
titles and bodies, scoped like the memory's entries by the identity of the
project's root folder, so removing a project and adding it again loses nothing.
The store announces each write (`StoreChange::Tasks`) so a window reading it
can refresh.
