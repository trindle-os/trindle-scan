# Security policy

Trindle Scan is a small project kept up in the time available, not a team with a dedicated security function. This page sets realistic expectations.

## Supported versions

Only the latest release is supported. There is no long-term-support branch and no backporting. A fix lands in the next release and you are expected to update.

| Version | Supported |
|---|---|
| 0.1.x | yes |

## Reporting a vulnerability

**Please do not put the details of a security problem in a public issue.** A public report tells everyone, including people running an unpatched copy, before a fix exists.

Use GitHub's private vulnerability reporting on this repository, under **Security**, then **Report a vulnerability**. Only the maintainers can see it.

If you would rather not use GitHub, write to <distro@trindle.org> with "Trindle Scan security" in the subject line and say only that you have a security issue. Leave the details out of that first message and a private channel will be arranged.

## What to expect

Replies and fixes are best effort, not guaranteed against any service level. You will be told when the report is read, and when a fix is released.

## What is in scope

Scanners are looked for on the local network only, after the program is opened. A fault in how the program talks to a scanner, reads a scanner's reply, reads a picture or writes a file is in scope. A scanner that is itself faulty is not.
