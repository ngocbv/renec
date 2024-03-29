# SPL Token Wallet

Note. This repository is used as a developer tool. Non security related issues and PRs will not be actively supported. For a wallet, it's recommended to use Phantom or Solflare.

See [sollet.io](https://www.sollet.io) or the [Sollet Chrome Extension](https://chrome.google.com/webstore/detail/sollet/fhmfendgdocmcbmfikdcogofphimnkno) for a demo.

Wallet keys are stored in `localStorage`, optionally encrypted by a password.

Run `yarn start` to start a development server or `yarn build` to create a production build that can be served by a static file server.

See the [Create React App documentation](https://facebook.github.io/create-react-app/docs/getting-started) for other commands and options.

### E2E testing

* run `yarn start` to start development server OR run `yarn build` and serve production build at localhost:3000
* run `yarn e2e-test` to open e2e test with GUI
* Or run `yarn e2e-test:headless` to run e2e without GUI
