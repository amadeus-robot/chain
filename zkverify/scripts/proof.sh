#!/bin/bash
set -e
mkdir -p build

echo "Compiling circuit..."
npx circom2 circuits/merkle.circom --r1cs --wasm --sym -o build > /dev/null

echo "Generating test data..."
node scripts/test.js > /dev/null

echo "Setting up ceremony..."
if [ ! -f "build/pot19_final.ptau" ]; then
    npx snarkjs powersoftau new bn128 19 build/pot19.ptau > /dev/null
    npx snarkjs powersoftau contribute build/pot19.ptau build/pot19_c.ptau --name="C" -e="r" > /dev/null
    npx snarkjs powersoftau prepare phase2 build/pot19_c.ptau build/pot19_final.ptau > /dev/null
fi

echo "Generating keys..."
if [ ! -f "build/merkle_final.zkey" ]; then
    npx snarkjs groth16 setup build/merkle.r1cs build/pot19_final.ptau build/merkle.zkey > /dev/null
    npx snarkjs zkey contribute build/merkle.zkey build/merkle_final.zkey --name="F" -e="r" > /dev/null
    npx snarkjs zkey export verificationkey build/merkle_final.zkey build/verification_key.json > /dev/null
else
    echo "  (using cached keys)"
fi

echo "Creating proof..."
npx snarkjs groth16 prove build/merkle_final.zkey build/witness.wtns build/proof.json build/public.json > /dev/null

echo "✅ Proof: $(wc -c < build/proof.json) bytes → build/proof.json"
echo ""
echo "Verify: npm run verify"
