# Repository implementation guidance

## Adding features

- Before implementing or reimplementing a feature, look for an existing implementation in nautilus File Manager or the kde file manager and study the relevant code or documentation. If it is unavailable, inspect a suitable established example and the closest pattern in this repository. Use examples to inform the design, then adapt and verify the approach against this app's architecture and requirements rather than copying blindly.
- Implement each new feature in its own focused source file or module. Keep feature UI, state, and behavior together in that feature module where practical.
- Touch existing files only for the smallest necessary integration points, such as registering a module, adding a feature state field, or mounting the feature component in the app.
- Do not place new feature UI inside an unrelated existing component, toolbar, file row, or folder view. Mount it at the appropriate location through a small, explicit integration call.
- Update `FEATURE_README.md` whenever a feature is added, changed, or removed so its status and notes match the code.

## Reusable components and functions

- Build UI from reusable components. Give each meaningful UI region a focused component function in its feature module instead of embedding a large one-off UI tree in an unrelated render function.
- When a feature needs multiple related UI pieces, split them into small component functions that can be composed and reused. Pass state and callbacks explicitly so components do not duplicate behavior.
- Keep non-UI functions focused and reusable as well: extract shared behavior into named helpers instead of repeating logic or growing large functions.
- Avoid abstractions that add indirection without reuse. Reuse should come from clear component boundaries and shared behavior, not from wrapping trivial one-use expressions unnecessarily.
- Keep names, ownership, and responsibilities clear; prefer small feature-specific modules over broad changes to existing files.
