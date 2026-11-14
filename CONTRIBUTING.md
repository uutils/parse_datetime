# Contributing

## Rerunning CI

If a pull request's CI needs to be run again without adding a commit, the pull
request author or a repository owner, member, or collaborator can comment
starting with:

```text
/retest
```

The command reruns every applicable pull-request CI workflow for the current
commit, including draft pull requests. Failed workflows rerun failed jobs.
