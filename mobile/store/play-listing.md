# Google Play listing

What goes into Play Console for `dev.zavu.leon`. The App Store listing is
`../store.config.json`, pushed with `eas metadata:push`; Play has no such
command, so this is pasted by hand.

## Main store listing

- **App name** (30): Leon - Coding Agents
- **Short description** (80): Drive Claude Code, Codex and opencode on your computer from your phone.
- **App icon**: `play-icon.png` (512 × 512)
- **Feature graphic**: `play-feature-graphic.png` (1024 × 500)
- **Phone screenshots**: `screenshots/android/*.png` (1080 × 1920), in order
- **Category**: Tools. **Tags**: Developer tools, Productivity
- **Website**: https://getleon.dev
- **Privacy policy**: https://getleon.dev/privacy

### Full description (4000)

Leon puts the coding agents running on your computer in your pocket. Start a task in Claude Code, Codex or opencode, walk away, and keep it moving from your phone.

SEND A PROMPT, GET THE ANSWER
Pick a project and an agent, type what you want, and follow every step as it happens: the files it reads, the commands it runs, the changes it makes.

APPROVE FROM ANYWHERE
When an agent asks for permission or has a question, you are told at once, even with the phone in your pocket. Allow, deny or tell it why, and it carries on.

EVERY SESSION, ON EVERY COMPUTER
Browse the sessions of each computer by project or by most recent, search them, and see the ones you pinned. Open any of them, read the conversation and continue it from the phone.

THE DEN
Every live agent as a lion: who is working, who is waiting for you, what each one is doing.

PRIVATE BY DESIGN
Pair with a QR code. The phone and the computer talk end-to-end encrypted, with keys only the two of them hold; the relay in between only passes along bytes it cannot read. No account, no analytics, no ads.

REQUIREMENTS
Leon for desktop (macOS, Linux or Windows), free at getleon.dev, with the agents you use installed. No computer at hand? Tap "Try a demo" to see the app with a made-up one.

## App content

- **Privacy policy**: https://getleon.dev/privacy
- **Ads**: No, the app contains no ads.
- **App access**: All or some functionality is restricted → add instructions:
  - Name: Demo computer
  - Username / password: leave empty
  - Instructions: Leon drives coding agents running on the user's own computer, which needs the Leon desktop app. To review without it, tap "Try a demo" on the first screen: it adds a simulated computer with sample projects, sessions and a working agent conversation, entirely on the device.
- **Content rating** (IARC questionnaire): category "All other app types"; answer Yes to "Online content" (it names generated AI content, and the app shows what the agents answer) and No to every other question (no violence, sexuality, language, drugs, gambling, user-to-user communication, location sharing, purchases). Rating given: all ages.
- **Target audience**: 18 and over. Not designed for children.
- **News app**: No.
- **Government app**: No.
- **Financial features**: none.
- **Health**: none.

### Data safety

- Does the app collect or share any of the required user data types? **No.**
  What the phone and the computer say to each other is end-to-end encrypted,
  and nothing is sent to Zavu; see the privacy policy.
- Is all user data encrypted in transit? **Yes.**
- Account creation: **the app does not let users create an account.**

### Foreground service (Play asks for each type the app declares)

- **Type**: Data sync (`FOREGROUND_SERVICE_DATA_SYNC`)
- **What it does**: While an agent is working on the user's computer, the app keeps its connection to that computer open, so that a question or permission request from the agent reaches the user at once, even with the screen off. The service runs only while a turn the user started is in flight, shows an ongoing notification saying so, and stops when the agent finishes.
- **User impact if deferred or interrupted**: The agent on the computer would wait for an answer the user is never told about, and the work the user asked for would stall.
- **Task**: Network processing → Other. The form asks only for this and the video; the two texts above are for when a reviewer asks.
- **Video**: a short screen recording of starting a turn, leaving the app, the ongoing notification, and answering the agent on the way back.
