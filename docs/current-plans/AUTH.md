# Authentication

We need to authenticate users in the system. This will involve setting up an auth method on creation, and confirmation
on sign-in.

As usual, when turning this into a plan:
1. The plan must be output to the docs/current-plans folder.
2. Each phase must start with the code style checklist using CLAUDE.md code style guidelines.
3. Each phase must start with the desired subagent to carry out tasks:
  - kiss-code-generator for simple, well-explained code steps in a single file (try to plan for this one the most often)
  - modular-builder for building out new files and cross-file work
4. Each phase must end with a reminder to run the FULL test suite, and upon 100% success update a status document with progress.
5. Each phase must include a precise list of files to be updated, created, or deleted. This will be the context for that
   phase.
6. Research the code and relevant libraries before coming up with the plan.
7. If after researching the code you have questions, ask for clarification.

## Decide on Auth method

1. Do we want Oauth2? Password authentication (with encryption of saved keys)? Let's discuss pros and cons. Actively
   work through this with the user.
2. This will start small (tiny batch of alpha users), but I don't want to change it drastically after that.
3. Initial step needs to be drastically restricted to only users to whom I give some kind of key (or whose email I
   verify, etc.)

## UserService

1. Create a new UserService on the backend to handle user creation, authentication, and authorization.
2. Rate limiter can be added to this.
3. Usual workflow: create a trait with the required methods, then implement a specific instance to follow that trait.

## User Creation

1. We will need to set up a full User Creation step instead of just letting people choose a username.
    - Username will still be part of setup. Usernames still need to be unique.
    - Email will become part of UserState to be set up.
    - Other Auth details depending on method chosen will need to be added
2. Instead of filling out a username for user creation, UI will need to display a button to create user.
3. This should go to user creation screen where users fill out all necessary information (username, email, auth info)
    - Keep it simple. If we don't need the information, don't ask for it. We don't want to collect anything more than
      necessary.
4. Make sure that data is persisted.
5. We don't have any real users yet (just me testing things out). Backwards compatibility and migration paths are not
   necessary. Blow it away and start from scratch.
6. Data goes to UserService on backend to confirm creation and persistence of new user.

## Sign-in

1. Sign-in similarly needs to go through user service.
2. Sign-in can be kept simple on frontend - username + whatever is necessary for auth.
3. UserService authenticates on backend. If successful, log in. If not, clear error message.
