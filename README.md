# Workspace Provider

The workspace provider is responsible for setting up a workspace for an agent to run tasks in.

## Motivation

When running our agents we want them to have a fresh environment that does not conflict with the environments of other agents that are running at the same time.

The manner in which we provision such an environment can vary depending on the circumstances. In a test it might be local or in Docker, for a customer it might be
in a specific cloud provider, etc.

In the workspace we want our agent to be able to run tasks.

Since provisioning workspaces is a fairly expensive operation, we want to provide whatever is useful for being able to undo or reset the workspace after a task has run.

## Special considerations

### Provisioning caching

The agent will (almost always) need to download the source code of the repository(s) that it is working on into the workspace. After that it will probably download
and/or install any dependencies. After these steps have been completed the workspace is in a clean state, and it would be a prime candidate for caching, so it
makes sense to have an explicit phase for the setup of the workspace.

Specifically for Git repositories, we could even reuse a cached workspace if there are changes in the repository if we know not to clone the repository again, but
instead to fetch the changes. This means the provider should be aware of which repositories are checked out and how to fetch changes for them.

### Secrets / configuration

It would be ideal if the workspace itself does not have access to any secrets, for example the authentication tokens for the git repositories. Instead, the provider
should be responsible for setting up the workspace before the agent gets access to it.

## Technical design

The workspace provider is started with the following arguments:

  - provisioning mode (e.g. local, docker, cloud)
  - repositories and their target paths, and authentication tokens
  - a setup script

Once the provider is started, it exposes a server that the agent controller can send requests to. The requests are:

  - setup workspace
  - teardown workspace
  - run commands in a workspace
  - fetch git changes in a workspace
