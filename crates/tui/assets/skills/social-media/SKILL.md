---
name: social-media
description: Read and analyze Messenger, Instagram, Threads, and Facebook via official data export, or the Graph API when tokened. Use when: messenger, instagram, threads, facebook, DMs, posts, or messages on social apps.
invocation: model+user
---

# Social Media

## When to use
Finding, reading, or summarizing the user's messages, posts, and activity on
Meta services: Messenger, Instagram, Threads, Facebook.

## Setup
Two paths, in this order:

1. **Official export (works for everyone).** The user downloads their data
   (Accounts Center → Download your information, JSON format), unzips it
   locally, and points you at the folder. Analyze `message_*.json`, posts,
   and media on disk. Fail loud when the folder is missing.
2. **Graph API (optional).** Only when the user already has a user token:
   `GET graph.facebook.com/v19.0/me/conversations`. Never walk the user
   through creating a Meta developer app unprompted — the export path first.

## Workflow
1. Establish which service and whose messages/posts are in scope.
2. Search the export for people, keywords, or date ranges; quote with dates.
3. Summarize threads; distinguish what the user wrote from what others wrote.

## Non-goals
- Do not post, send, like, follow, or delete anything on any service.
- Do not log into accounts or handle passwords/2FA codes.
- Do not retain or repeat other people's private messages beyond the task.
