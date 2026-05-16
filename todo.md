# TODO

## Completed

- [x] Check for an existing `todo.md` file and confirm the repository did not currently include one.
- [x] Extract Terraform JSON parsing into a dedicated `parse_plan_output` helper.
- [x] Replace ad-hoc `serde_json::Value` traversal with typed `serde` deserialization structs.
- [x] Add parser unit tests for newline-delimited Terraform JSON, invalid JSON lines, and missing actions.
- [x] Keep the architecture notes discoverable from the README.
- [x] Add more follow-up items to this TODO list.

## Follow-up

- [ ] Stream Terraform stdout directly from the child process instead of waiting for the full command output before parsing.
- [ ] Add CLI flags for structured output formats such as `--format json`, `--format csv`, and `--format table`.
- [ ] Add filtering flags such as `--include-type`, `--exclude-type`, `--include-action`, and `--exclude-action`.
- [ ] Support parsing saved `.tfplan` files in addition to running a live `terraform plan` command.
- [ ] Add integration tests that exercise CLI behavior with a mocked `terraform` executable.
