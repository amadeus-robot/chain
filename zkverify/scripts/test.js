const crypto = require("crypto");
const snarkjs = require("snarkjs");
const fs = require("fs");

// Convert byte array to bit array (MSB first)
function bytesToBits(bytes) {
    const bits = [];
    for (let i = 0; i < bytes.length; i++) {
        for (let j = 7; j >= 0; j--) {
            bits.push((bytes[i] >> j) & 1);
        }
    }
    return bits;
}

// Convert 32-byte hash to two 128-bit field elements (hi, lo)
function bytesToFieldElements(bytes) {
    // Split into high 16 bytes and low 16 bytes
    const hi = bytes.slice(0, 16);
    const lo = bytes.slice(16, 32);

    // Convert to BigInt (MSB first)
    let hiNum = 0n;
    let loNum = 0n;

    for (let i = 0; i < 16; i++) {
        hiNum = (hiNum << 8n) | BigInt(hi[i]);
        loNum = (loNum << 8n) | BigInt(lo[i]);
    }

    return [hiNum.toString(), loNum.toString()];
}

// SHA-256 hash function
function sha256(data) {
    return crypto.createHash("sha256").update(data).digest();
}

// Hash two 32-byte values together
function hashPair(left, right) {
    const combined = Buffer.concat([left, right]);
    return sha256(combined);
}

async function generateTestProof() {
    // Create 8 leaf values
    const leaves = [];
    for (let i = 0; i < 8; i++) {
        const value = Buffer.alloc(32);
        value.writeUInt32BE(1000 + i, 28); // Put value at the end
        leaves.push(value);
    }

    // Hash each leaf to get leaf hashes
    const level0 = leaves.map(l => sha256(l));

    // Build tree level by level
    const tree = [level0];

    let currentLevel = level0;
    for (let lvl = 0; lvl < 3; lvl++) {
        const nextLevel = [];
        for (let i = 0; i < currentLevel.length; i += 2) {
            const left = currentLevel[i];
            const right = currentLevel[i + 1];
            nextLevel.push(hashPair(left, right));
        }
        tree.push(nextLevel);
        currentLevel = nextLevel;
    }

    const root = tree[3][0];
    const leafIndex = 5;
    const leafValue = leaves[leafIndex];

    // Collect siblings for the proof
    const siblings = [];
    let currentIndex = leafIndex;

    for (let lvl = 0; lvl < 3; lvl++) {
        const isRight = currentIndex % 2 === 1;
        const siblingIndex = isRight ? currentIndex - 1 : currentIndex + 1;
        const sibling = tree[lvl][siblingIndex];
        siblings.push(sibling);
        currentIndex = Math.floor(currentIndex / 2);
    }

    // Convert root and leaf to field elements (hi, lo pairs)
    const [rootHi, rootLo] = bytesToFieldElements(root);
    const [leafHi, leafLo] = bytesToFieldElements(leafValue);

    // Convert to bit arrays for circuit (private inputs)
    const input = {
        rootHi: rootHi,
        rootLo: rootLo,
        leafHi: leafHi,
        leafLo: leafLo,
        siblings: siblings.map(s => bytesToBits(s)),
        indices: []
    };

    // Calculate indices (path bits)
    currentIndex = leafIndex;
    for (let lvl = 0; lvl < 3; lvl++) {
        const isRight = currentIndex % 2 === 1;
        input.indices.push(isRight ? "1" : "0");
        currentIndex = Math.floor(currentIndex / 2);
    }

    fs.mkdirSync("build", { recursive: true });
    fs.writeFileSync("build/input.json", JSON.stringify(input, null, 2));

    console.log("Test data generated:");
    console.log("  Root:", root.toString("hex"));
    console.log("  Leaf:", leafValue.toString("hex"));
    console.log("  Leaf index:", leafIndex);
    console.log("  Indices:", input.indices.join(", "));
    console.log("  Public inputs: rootHi, rootLo, leafHi, leafLo (4 field elements)");

    await snarkjs.wtns.calculate(input, "build/merkle_js/merkle.wasm", "build/witness.wtns");
    console.log("✓ Test data and witness generated");
}

generateTestProof().catch(console.error);
