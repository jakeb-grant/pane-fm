import hljs from "highlight.js/lib/core";
import langBash from "highlight.js/lib/languages/bash";
import langC from "highlight.js/lib/languages/c";
import langClojure from "highlight.js/lib/languages/clojure";
import langCmake from "highlight.js/lib/languages/cmake";
import langCpp from "highlight.js/lib/languages/cpp";
import langCsharp from "highlight.js/lib/languages/csharp";
import langCss from "highlight.js/lib/languages/css";
import langDart from "highlight.js/lib/languages/dart";
import langDiff from "highlight.js/lib/languages/diff";
import langDockerfile from "highlight.js/lib/languages/dockerfile";
import langElixir from "highlight.js/lib/languages/elixir";
import langErlang from "highlight.js/lib/languages/erlang";
import langGo from "highlight.js/lib/languages/go";
import langGraphql from "highlight.js/lib/languages/graphql";
import langHaskell from "highlight.js/lib/languages/haskell";
import langIni from "highlight.js/lib/languages/ini";
import langJava from "highlight.js/lib/languages/java";
import langJavascript from "highlight.js/lib/languages/javascript";
import langJson from "highlight.js/lib/languages/json";
import langKotlin from "highlight.js/lib/languages/kotlin";
import langLess from "highlight.js/lib/languages/less";
import langLisp from "highlight.js/lib/languages/lisp";
import langLua from "highlight.js/lib/languages/lua";
import langMakefile from "highlight.js/lib/languages/makefile";
import langMarkdown from "highlight.js/lib/languages/markdown";
import langNim from "highlight.js/lib/languages/nim";
import langNix from "highlight.js/lib/languages/nix";
import langOcaml from "highlight.js/lib/languages/ocaml";
import langPerl from "highlight.js/lib/languages/perl";
import langPhp from "highlight.js/lib/languages/php";
import langPlaintext from "highlight.js/lib/languages/plaintext";
import langProtobuf from "highlight.js/lib/languages/protobuf";
import langPython from "highlight.js/lib/languages/python";
import langR from "highlight.js/lib/languages/r";
import langRuby from "highlight.js/lib/languages/ruby";
import langRust from "highlight.js/lib/languages/rust";
import langScala from "highlight.js/lib/languages/scala";
import langScss from "highlight.js/lib/languages/scss";
import langSql from "highlight.js/lib/languages/sql";
import langSwift from "highlight.js/lib/languages/swift";
import langTypescript from "highlight.js/lib/languages/typescript";
import langVim from "highlight.js/lib/languages/vim";
import langX86asm from "highlight.js/lib/languages/x86asm";
import langXml from "highlight.js/lib/languages/xml";
import langYaml from "highlight.js/lib/languages/yaml";

hljs.registerLanguage("bash", langBash);
hljs.registerLanguage("c", langC);
hljs.registerLanguage("clojure", langClojure);
hljs.registerLanguage("cmake", langCmake);
hljs.registerLanguage("cpp", langCpp);
hljs.registerLanguage("csharp", langCsharp);
hljs.registerLanguage("css", langCss);
hljs.registerLanguage("dart", langDart);
hljs.registerLanguage("diff", langDiff);
hljs.registerLanguage("dockerfile", langDockerfile);
hljs.registerLanguage("elixir", langElixir);
hljs.registerLanguage("erlang", langErlang);
hljs.registerLanguage("go", langGo);
hljs.registerLanguage("graphql", langGraphql);
hljs.registerLanguage("haskell", langHaskell);
hljs.registerLanguage("ini", langIni);
hljs.registerLanguage("java", langJava);
hljs.registerLanguage("javascript", langJavascript);
hljs.registerLanguage("json", langJson);
hljs.registerLanguage("kotlin", langKotlin);
hljs.registerLanguage("less", langLess);
hljs.registerLanguage("lisp", langLisp);
hljs.registerLanguage("lua", langLua);
hljs.registerLanguage("makefile", langMakefile);
hljs.registerLanguage("markdown", langMarkdown);
hljs.registerLanguage("nim", langNim);
hljs.registerLanguage("nix", langNix);
hljs.registerLanguage("ocaml", langOcaml);
hljs.registerLanguage("perl", langPerl);
hljs.registerLanguage("php", langPhp);
hljs.registerLanguage("plaintext", langPlaintext);
hljs.registerLanguage("protobuf", langProtobuf);
hljs.registerLanguage("python", langPython);
hljs.registerLanguage("r", langR);
hljs.registerLanguage("ruby", langRuby);
hljs.registerLanguage("rust", langRust);
hljs.registerLanguage("scala", langScala);
hljs.registerLanguage("scss", langScss);
hljs.registerLanguage("sql", langSql);
hljs.registerLanguage("swift", langSwift);
hljs.registerLanguage("typescript", langTypescript);
hljs.registerLanguage("vim", langVim);
hljs.registerLanguage("x86asm", langX86asm);
hljs.registerLanguage("xml", langXml);
hljs.registerLanguage("yaml", langYaml);

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
	fish: "bash",
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
