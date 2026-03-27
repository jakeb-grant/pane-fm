import hljs from "highlight.js";

export interface HighlightRequest {
	code: string;
	filename: string;
	gen: number;
}

export interface HighlightResponse {
	html: string;
	gen: number;
}

export const extToLang: Record<string, string> = {
	js: "javascript",
	mjs: "javascript",
	cjs: "javascript",
	ts: "typescript",
	jsx: "javascript",
	tsx: "typescript",
	py: "python",
	rb: "ruby",
	rs: "rust",
	go: "go",
	java: "java",
	kt: "kotlin",
	kts: "kotlin",
	c: "c",
	cpp: "cpp",
	cc: "cpp",
	cxx: "cpp",
	h: "c",
	hpp: "cpp",
	hxx: "cpp",
	cs: "csharp",
	swift: "swift",
	sh: "bash",
	bash: "bash",
	zsh: "bash",
	fish: "fish",
	html: "xml",
	htm: "xml",
	xml: "xml",
	svg: "xml",
	css: "css",
	scss: "scss",
	less: "less",
	json: "json",
	yaml: "yaml",
	yml: "yaml",
	toml: "ini",
	ini: "ini",
	conf: "ini",
	md: "markdown",
	sql: "sql",
	lua: "lua",
	r: "r",
	php: "php",
	pl: "perl",
	pm: "perl",
	ex: "elixir",
	exs: "elixir",
	erl: "erlang",
	hs: "haskell",
	ml: "ocaml",
	mli: "ocaml",
	clj: "clojure",
	cljs: "clojure",
	scala: "scala",
	dart: "dart",
	zig: "zig",
	nim: "nim",
	nix: "nix",
	vim: "vim",
	el: "lisp",
	lisp: "lisp",
	cmake: "cmake",
	diff: "diff",
	patch: "diff",
	dockerfile: "dockerfile",
	proto: "protobuf",
	graphql: "graphql",
	gql: "graphql",
	tf: "hcl",
	hcl: "hcl",
	asm: "x86asm",
	s: "x86asm",
};

export const nameToLang: Record<string, string> = {
	makefile: "makefile",
	gnumakefile: "makefile",
	dockerfile: "dockerfile",
	containerfile: "dockerfile",
	cmakelists: "cmake",
	rakefile: "ruby",
	gemfile: "ruby",
	vagrantfile: "ruby",
	justfile: "makefile",
};

const AUTO_SUBSET = [
	"javascript",
	"typescript",
	"python",
	"bash",
	"json",
	"xml",
	"css",
	"sql",
	"rust",
	"go",
	"java",
	"cpp",
	"c",
	"ruby",
	"php",
	"markdown",
	"yaml",
	"ini",
];

export function highlightCode(code: string, filename: string): string {
	const lower = filename.toLowerCase();
	const baseName = lower.split("/").pop() ?? lower;
	const ext = baseName.includes(".") ? baseName.split(".").pop()! : "";

	const lang = extToLang[ext] || nameToLang[baseName.replace(/\..+$/, "")];

	if (lang && hljs.getLanguage(lang)) {
		return hljs.highlight(code, { language: lang }).value;
	}
	const auto = hljs.highlightAuto(code, AUTO_SUBSET);
	if (auto.relevance > 5) {
		return auto.value;
	}
	return hljs.highlight(code, { language: "plaintext" }).value;
}
