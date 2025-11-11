pragma circom 2.0.0;

include "../node_modules/circomlib/circuits/sha256/sha256.circom";
include "../node_modules/circomlib/circuits/bitify.circom";

// Helper template to select between two 256-bit values based on a selector
template Mux256() {
    signal input left[256];
    signal input right[256];
    signal input sel;  // 0 = output left, 1 = output right
    signal output out[256];

    for (var i = 0; i < 256; i++) {
        out[i] <== left[i] + sel * (right[i] - left[i]);
    }
}

template MerkleProof() {
    // Public inputs as field elements (more compact)
    signal input rootHi;      // High 128 bits of root as field element
    signal input rootLo;      // Low 128 bits of root as field element
    signal input leafHi;      // High 128 bits of leaf as field element
    signal input leafLo;      // Low 128 bits of leaf as field element

    // Private inputs
    signal input siblings[3][256]; // 3 siblings, each 256 bits
    signal input indices[3];       // path indices (0 or 1)

    // Convert public field elements back to bit arrays
    component rootHiBits = Num2Bits(128);
    component rootLoBits = Num2Bits(128);
    component leafHiBits = Num2Bits(128);
    component leafLoBits = Num2Bits(128);

    rootHiBits.in <== rootHi;
    rootLoBits.in <== rootLo;
    leafHiBits.in <== leafHi;
    leafLoBits.in <== leafLo;

    // Reconstruct full 256-bit arrays (MSB first)
    signal rootBits[256];
    signal leafBits[256];

    for (var i = 0; i < 128; i++) {
        rootBits[i] <== rootHiBits.out[127-i];
        rootBits[i+128] <== rootLoBits.out[127-i];
        leafBits[i] <== leafHiBits.out[127-i];
        leafBits[i+128] <== leafLoBits.out[127-i];
    }

    // Hash the leaf value
    component leafHash = Sha256(256);
    for (var i = 0; i < 256; i++) {
        leafHash.in[i] <== leafBits[i];
    }

    // Level 0: hash(leafHash, sibling[0])
    component mux0_left = Mux256();
    component mux0_right = Mux256();
    for (var i = 0; i < 256; i++) {
        mux0_left.left[i] <== leafHash.out[i];
        mux0_left.right[i] <== siblings[0][i];
        mux0_right.left[i] <== siblings[0][i];
        mux0_right.right[i] <== leafHash.out[i];
    }
    mux0_left.sel <== indices[0];
    mux0_right.sel <== indices[0];

    component hash0 = Sha256(512);
    for (var i = 0; i < 256; i++) {
        hash0.in[i] <== mux0_left.out[i];
        hash0.in[i + 256] <== mux0_right.out[i];
    }

    // Level 1: hash(hash0, sibling[1])
    component mux1_left = Mux256();
    component mux1_right = Mux256();
    for (var i = 0; i < 256; i++) {
        mux1_left.left[i] <== hash0.out[i];
        mux1_left.right[i] <== siblings[1][i];
        mux1_right.left[i] <== siblings[1][i];
        mux1_right.right[i] <== hash0.out[i];
    }
    mux1_left.sel <== indices[1];
    mux1_right.sel <== indices[1];

    component hash1 = Sha256(512);
    for (var i = 0; i < 256; i++) {
        hash1.in[i] <== mux1_left.out[i];
        hash1.in[i + 256] <== mux1_right.out[i];
    }

    // Level 2: hash(hash1, sibling[2])
    component mux2_left = Mux256();
    component mux2_right = Mux256();
    for (var i = 0; i < 256; i++) {
        mux2_left.left[i] <== hash1.out[i];
        mux2_left.right[i] <== siblings[2][i];
        mux2_right.left[i] <== siblings[2][i];
        mux2_right.right[i] <== hash1.out[i];
    }
    mux2_left.sel <== indices[2];
    mux2_right.sel <== indices[2];

    component hash2 = Sha256(512);
    for (var i = 0; i < 256; i++) {
        hash2.in[i] <== mux2_left.out[i];
        hash2.in[i + 256] <== mux2_right.out[i];
    }

    // Verify root matches by comparing bit arrays
    for (var i = 0; i < 256; i++) {
        rootBits[i] === hash2.out[i];
    }
}

component main {public [rootHi, rootLo, leafHi, leafLo]} = MerkleProof();
