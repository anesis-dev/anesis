#!/usr/bin/env node
"use strict";

const https = require("https");
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");
const { spawnSync } = require("child_process");

const PLATFORM_MAP = {
	"linux-x64": { name: "linux-x86_64", ext: "tar.gz", binary: "anesis" },
	"linux-arm64": { name: "linux-aarch64", ext: "tar.gz", binary: "anesis" },
	"darwin-arm64": { name: "macos-aarch64", ext: "tar.gz", binary: "anesis" },
	"win32-x64": { name: "windows-x86_64", ext: "zip", binary: "anesis.exe" },
};

const key = `${process.platform}-${process.arch}`;
const platform = PLATFORM_MAP[key];
if (!platform) {
	console.error(
		`anesis: unsupported platform "${key}". ` +
			`Supported via npm: linux-x64, linux-arm64, darwin-arm64, win32-x64. ` +
			`Install manually from https://github.com/anesis-dev/anesis/releases`,
	);
	process.exit(1);
}

const pkg = JSON.parse(
	fs.readFileSync(path.join(__dirname, "package.json"), "utf8"),
);
const version = pkg.binaryVersion || pkg.version;

const binDir = path.join(__dirname, "bin");
const dest = path.join(binDir, platform.binary);
const versionFile = path.join(binDir, ".version");

if (
	fs.existsSync(dest) &&
	fs.existsSync(versionFile) &&
	fs.readFileSync(versionFile, "utf8").trim() === version
) {
	process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

const assetName = `anesis-${platform.name}.${platform.ext}`;
const url = `https://github.com/anesis-dev/anesis/releases/download/v${version}/${assetName}`;

console.log(`anesis: downloading ${url}`);

function download(url, cb) {
	https
		.get(url, { headers: { "User-Agent": "anesis-npm-installer" } }, (res) => {
			if (
				res.statusCode >= 301 &&
				res.statusCode <= 308 &&
				res.headers.location
			) {
				return download(res.headers.location, cb);
			}
			if (res.statusCode !== 200) {
				cb(new Error(`HTTP ${res.statusCode} downloading ${url}`));
				return;
			}
			const chunks = [];
			res.on("data", (c) => chunks.push(c));
			res.on("end", () => cb(null, Buffer.concat(chunks)));
			res.on("error", cb);
		})
		.on("error", cb);
}

function extractTarGz(buf, destPath) {
	const inflated = zlib.gunzipSync(buf);
	let offset = 0;
	while (offset + 512 <= inflated.length) {
		const header = inflated.slice(offset, offset + 512);
		const name = header.slice(0, 100).toString("utf8").replace(/\0.*$/, "");
		if (!name) break;
		const sizeStr = header
			.slice(124, 136)
			.toString("utf8")
			.replace(/\0.*$/, "")
			.trim();
		const size = parseInt(sizeStr, 8) || 0;
		const typeFlag = header[156];
		offset += 512;
		// Regular file (type '0' = 0x30, or NUL for old-style)
		if (typeFlag === 0x30 || typeFlag === 0) {
			if (path.basename(name) === "anesis") {
				fs.writeFileSync(destPath, inflated.slice(offset, offset + size), {
					mode: 0o755,
				});
				console.log(`anesis: installed to ${destPath}`);
				return;
			}
		}
		offset += Math.ceil(size / 512) * 512;
	}
	throw new Error("anesis binary not found in archive");
}

function extractZip(buf, destPath) {
	// Locate End of Central Directory (signature 0x06054b50)
	let eocdOffset = -1;
	for (let i = buf.length - 22; i >= 0; i--) {
		if (buf.readUInt32LE(i) === 0x06054b50) {
			eocdOffset = i;
			break;
		}
	}
	if (eocdOffset === -1) throw new Error("Invalid ZIP: EOCD not found");

	const cdEntries = buf.readUInt16LE(eocdOffset + 8);
	const cdOffset = buf.readUInt32LE(eocdOffset + 16);

	let pos = cdOffset;
	for (let i = 0; i < cdEntries; i++) {
		if (buf.readUInt32LE(pos) !== 0x02014b50)
			throw new Error("Invalid ZIP central directory");
		const method = buf.readUInt16LE(pos + 10);
		const compressedSz = buf.readUInt32LE(pos + 20);
		const fileNameLen = buf.readUInt16LE(pos + 28);
		const extraLen = buf.readUInt16LE(pos + 30);
		const commentLen = buf.readUInt16LE(pos + 32);
		const localOffset = buf.readUInt32LE(pos + 42);
		const fileName = buf
			.slice(pos + 46, pos + 46 + fileNameLen)
			.toString("utf8");
		pos += 46 + fileNameLen + extraLen + commentLen;

		if (path.basename(fileName) === "anesis.exe") {
			if (buf.readUInt32LE(localOffset) !== 0x04034b50)
				throw new Error("Invalid ZIP local header");
			const localFileNameLen = buf.readUInt16LE(localOffset + 26);
			const localExtraLen = buf.readUInt16LE(localOffset + 28);
			const dataOffset = localOffset + 30 + localFileNameLen + localExtraLen;
			const compressed = buf.slice(dataOffset, dataOffset + compressedSz);

			const data = method === 0 ? compressed : zlib.inflateRawSync(compressed);
			fs.writeFileSync(destPath, data);
			console.log(`anesis: installed to ${destPath}`);
			return;
		}
	}
	throw new Error("anesis.exe not found in ZIP archive");
}

download(url, (err, buf) => {
	if (err) {
		console.error(`anesis: download failed: ${err.message}`);
		process.exit(1);
	}
	try {
		if (platform.ext === "tar.gz") {
			extractTarGz(buf, dest);
		} else {
			extractZip(buf, dest);
		}
		fs.writeFileSync(versionFile, version);
		installCompletions(dest);
	} catch (e) {
		console.error(`anesis: extraction failed: ${e.message}`);
		process.exit(1);
	}
});

function installCompletions(binaryPath) {
	const shell =
		process.platform === "win32"
			? "powershell"
			: path.basename(process.env.SHELL || "");
	if (!shell || !["bash", "zsh", "fish", "powershell"].includes(shell)) return;
	try {
		spawnSync(binaryPath, ["completions", shell], { stdio: "inherit" });
	} catch (_) {
		// completions are optional — never fail the install
	}
}
