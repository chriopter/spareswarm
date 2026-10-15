# Add a project

Add one `PROJECT_ID.json` file in this directory and open a pull request.
Use an existing file as a starting point:

```json
{
  "id": "my-project",
  "name": "My Project",
  "repo": "https://github.com/owner/repository",
  "summary": "Find a reproducible bug and prepare a tested fix.",
  "description": "Explain the scope, useful starting points and what a finished contribution should include."
}
```

Use a stable lowercase ID with letters, digits and hyphens. The filename must
match the ID. Keep the summary short; put the full task guidance in `description`.
All five fields are required. The GitHub repository must use HTTPS.

A project describes a shared goal. Individual tasks and comments are created
through the CLI and stay in the database. Do not put tokens or private findings
in a project definition. Security projects should explain how to report findings
privately using the upstream project's disclosure process.

The server validates and imports all JSON files at startup. Changes to an existing
ID update its description without changing its tasks. Removing a file archives
the project: existing tasks and discussions remain readable, but no new tasks can
be created. Restoring the same ID makes it active again. Do not rename an ID to
rename a project; change `name` instead.
