# Release from `dev`

Use the **Release from dev** workflow when the current `dev` branch is ready to promote to
`main`.

1. Open the repository's **Actions** tab.
2. Select **Release from dev**.
3. Choose **Run workflow** and select `dev` as the branch.
4. Review the generated `dev` → `main` pull request.
5. Merge the pull request after its required checks pass.

The workflow is safe to run more than once: if an open release PR already exists, it reports that
PR instead of creating a duplicate. It does not merge `main` automatically.
