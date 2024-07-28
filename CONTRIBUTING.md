# contributing

============
We are all living beings, and what is most important is that we respect each other and work together. If you can not uphold this simple standard, then your contributions are not welcome.

## hacking

Just a few simple items to keep in mind as you hack.

- Pull request early and often. This helps to let others know what you are working on. **Please use GitHub's Draft PR mechanism** if your PR is not yet ready for review.
- Use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/), a changelog will automatically be created from such commits
- When making changes to the configuration, be sure to regenate the schema. This can be done by running:

  ```shell
  cargo run -- config generate-schema schemas/config.json
  ```

## linting

We are using clippy & rustfmt. Clippy is SO GREAT! Rustfmt ... has a lot more growing to do; however, we are using it for uniformity.

Please be sure that you've configured your editor to use clippy & rustfmt, or execute them manually before submitting your code. CI will fail your PR if you do not.
