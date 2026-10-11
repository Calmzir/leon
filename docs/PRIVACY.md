# Privacy

Published at <https://getleon.dev/privacy>. Leon is made by Zavu. This page covers the Leon desktop application and the
Leon phone app (iPhone and Android). Last updated: October 11, 2026.

## In short

Leon has no accounts, no analytics, no advertising and no tracking. What you
do with your agents stays on your computers and your phone.

## What stays on your devices

* **On your computer**: your projects, your agents' sessions and their
  history, and the list of devices you allowed to connect.
* **On your phone**: the keys that identify it, the computers you paired with,
  and the chats you started or opened from the phone.

None of this is sent to Zavu. Removing a computer in the phone app deletes its
chats from the phone; deleting the app deletes everything it kept.

## The relay

A phone and a computer that are not on the same network reach each other
through a relay server. Zavu runs one at `relay.getleon.dev`; you can run your
own and point both devices at it.

Everything the two devices say to each other is end-to-end encrypted with keys
only they hold. The relay passes along bytes it cannot read: it never sees
your prompts, your agents' answers, your files, commands, keys or device names.

To do its job the relay sees connection metadata: the network address each
device connects from, that a computer with a given random identifier is
online, when devices connect and leave, and how much data flows. Zavu uses
this only to run the relay and keep it working, and does not sell or share it.

## The camera

The phone app uses the camera only to read the pairing code shown on your
computer. The image is read on the phone and is neither stored nor sent
anywhere.

## Notifications

The phone app shows notifications that the phone itself raises, such as an
agent asking for your permission. They are not sent through any push service.

## Images of projects

To show the logo of a project, the phone may load the public avatar of the
repository's owner from GitHub, as a web browser would.

## Your agents

The coding agents you drive with Leon (such as Claude Code, Codex or opencode)
run on your computer and talk to their own providers under those providers'
terms and privacy policies. Leon does not change what they send.

## Children

Leon is a tool for software developers and is not directed at children.

## Changes and contact

If this policy changes, this page changes with it. Questions:
open an issue at <https://github.com/zavudev/leon/issues>.
