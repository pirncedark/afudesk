# AfuDesk v1.10.8 - Terminal hold-to-talk key repeat

- Fixed: holding the terminal microphone now sends repeated Space DOWN events after a 500 ms warmup, every 50 ms. No intermediate Space UP is sent.
- Release, cancellation, session close, disconnect and backgrounding stop repeats and release Space.
- Terminal tooltips and the introductory terminal hint are hidden.
- Ctrl+C and Ctrl+P are preserved. Android version: 1.10.8+22.

Claude Code hold mode detects rapid terminal key-repeat events, rather than a single key-down state: https://code.claude.com/docs/en/voice-dictation#hold-to-record

This change controls the remote keyboard. It does not transmit the phone microphone to the computer. Live phone-to-computer audio recording has not been verified.
