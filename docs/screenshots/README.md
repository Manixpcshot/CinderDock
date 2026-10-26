# Screenshots

These images are generated from the production CinderDock frontend with:

```bash
npm run screenshots
```

The script builds the app, starts a local Vite preview, injects a non-sensitive fixture backend through the app's development event/invoke seam, and captures the real React components with headless Chromium. It contains no API keys, personal chats, or real provider credentials.

| File | View |
| --- | --- |
| `01-launch-rtl.png` | Native Persian RTL launch screen |
| `02-conversation-activity.png` | Conversation, tool activity and terminal output |
| `03-file-diff.png` | Applied file change and unified diff |
| `04-approval.png` | Detailed approval dialog with diff |
| `05-provider-connection.png` | Provider connection wizard |
| `06-model-manager.png` | Searchable model manager with multiple providers |
| `07-finglish-latin.png` | Finglish/Latin display mode with LTR layout |
