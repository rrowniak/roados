# System architecture

## Intro

This is the RoadOS system architecture description.

Different part of the system are written to separate files, the convention is `roados.<module>.<submodule>...md`. There are exceptions:

- roados.md - this file, the high level description.
- roados.hardware.md - this is a hardware specification, info about cross-compiling, etc.
- roados.undefined.<module-submodule>.md - we need to decide where given module (and submodule) belong to.

## High level architecture

```
-------------           ----------------         ----------------
| core       |          | infotainment  |        |  head up      |
|            |  ======> |               |        |               |
|            |          -----------------        |               |
|            |  ==============================>  |               |
--------------                                   -----------------
|----------------------------------------|       |----------------|
|     Linux os + CM4 (5) + LCD 15.6"     |       | To be defined  |
|                                        |       |                |
------------------------------------------       ------------------
```
