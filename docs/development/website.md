# Project Website

Status: Current

The public project website is a static site under `site/`. It is deployed from `main` with GitHub Actions and does not use a separate `gh-pages` branch.

## Local Preview

From the repository root:

```sh
python3 -m http.server 8000 --directory site
```

Open `http://localhost:8000`. A static server is preferred to opening `site/index.html` directly because it matches GitHub Pages URL and asset behavior more closely.

## Deployment

`.github/workflows/pages.yml` runs when `site/**` or the workflow itself changes on `main`. It uploads the `site/` directory as a Pages artifact and deploys that artifact to the `github-pages` environment.

The source remains on `main`; deployment artifacts are owned by GitHub Pages. This keeps site code reviewable beside the project without maintaining generated files in another branch.

## Custom Domain

The default address is provided by GitHub Pages. A custom address requires control of a domain, usually purchased through a domain registrar, plus DNS records pointing to GitHub Pages. Configure the domain in the repository Pages settings and add a `site/CNAME` file only when the final domain has been chosen.

HTTPS should remain enabled after GitHub verifies the DNS configuration.
