# JS Bindings for VecPak

### Usage

```js
//  Add to package.json
// "@amadeus-protocol/vecpak-js": "https://gitpkg.now.sh/amadeusprotocol/chain/vecpak/bindings/js?main"

import { encode, decode } from "@amadeus-protocol/vecpak-js";

const tx = {
	signer: globalState.pk,
	nonce: window.BigInt(Date.now()) * 1_000_000n,
	actions: [{op: "call", contract: contract, function: func, args: args}]
}
const tx_encoded = encode(tx);
```
