# Security policy

## Supported versions

Security fixes are applied to the latest version on the default branch.

## Reporting a vulnerability

Please use this repository's private vulnerability reporting feature. Do not
open a public issue for a suspected security vulnerability. Include a clear
description, reproduction steps, impact, and any suggested mitigation.

A valid report will be acknowledged within seven days, and the fix and
disclosure timeline will be agreed with the reporter.

## What this project does and does not do

DrDebits is a source-linked ethics guide plus a Python build that renders it.
The build and verify jobs read and write local files. The weekly link check
fetches public URLs and reports failures; it does not send credentials.

Do not treat the guide as a substitute for the TPB, APESB or AUSTRAC sources,
and do not load untrusted files into the build as if they were `src/`.
