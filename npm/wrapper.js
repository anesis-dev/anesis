#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");
const path = require("path");

const bin = path.join(
	__dirname,
	"bin",
	process.platform === "win32" ? "anesis.exe" : "anesis",
);

const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
	if (result.error.code === "ENOENT") {
		console.error(
			"anesis: binary not found. Try reinstalling: npm install -g @anesis/anesis",
		);
	} else {
		console.error(`anesis: ${result.error.message}`);
	}
	process.exit(1);
}

process.exit(result.status ?? 1);
