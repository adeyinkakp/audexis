import { createPublicKey, createHash, verify } from "node:crypto";
import { readFileSync } from "node:fs";

const [publicKeyFile, file] = process.argv.slice(2);
const decode = (value) =>
  Buffer.from(value.trim(), "base64").toString("utf8").trimEnd().split("\n");
const publicLines = decode(readFileSync(publicKeyFile, "utf8"));
const signatureLines = decode(readFileSync(`${file}.sig`, "utf8"));
const publicPacket = Buffer.from(publicLines[1], "base64");
const signaturePacket = Buffer.from(signatureLines[1], "base64");
if (
  publicPacket.length !== 42 ||
  signaturePacket.length !== 74 ||
  !publicPacket.subarray(2, 10).equals(signaturePacket.subarray(2, 10))
) {
  throw new Error(
    "Updater signing key does not match apps/desktop/updater.pub",
  );
}
const key = createPublicKey({
  key: Buffer.concat([
    Buffer.from("302a300506032b6570032100", "hex"),
    publicPacket.subarray(10),
  ]),
  format: "der",
  type: "spki",
});
const algorithm = signaturePacket.subarray(0, 2).toString("ascii");
if (!["Ed", "ED"].includes(algorithm))
  throw new Error("Unsupported minisign signature");
const data = readFileSync(file);
const message =
  algorithm === "ED" ? createHash("blake2b512").update(data).digest() : data;
const signature = signaturePacket.subarray(10);
const trustedPrefix = "trusted comment: ";
if (
  !signatureLines[2]?.startsWith(trustedPrefix) ||
  !verify(null, message, key, signature) ||
  !verify(
    null,
    Buffer.concat([
      signature,
      Buffer.from(signatureLines[2].slice(trustedPrefix.length)),
    ]),
    key,
    Buffer.from(signatureLines[3], "base64"),
  )
) {
  throw new Error("Updater signature verification failed");
}
console.log(`Verified updater signature: ${file}`);
