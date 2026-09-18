import { describe, expect, it } from "vitest";
import type { ModEntry } from "../ipc/commands";
import { buildExport, parseExport, verifyExport } from "./sync";

const mod = (key: string, file: string, size: number, source: ModEntry["source"] = "data"): ModEntry => ({
  key,
  file,
  path: file,
  dir: "",
  source,
  workshopId: source === "workshop" ? key.slice(3, key.indexOf("/")) : null,
  packType: "mod",
  size,
  mtime: 0,
  previewPath: null,
});

const H = (c: string) => c.repeat(64);

describe("profile export", () => {
  it("round-trips v2 and still reads v1", () => {
    const text = buildExport("MP", [
      { key: "ws:1/a.pack", file: "a.pack", size: 10, sha256: H("a") },
      { key: "data:b c.pack", file: "b c.pack", size: 20, sha256: H("b") },
    ]);
    expect(parseExport(text)).toEqual([
      { key: "ws:1/a.pack", file: "a.pack", size: 10, sha256: H("a") },
      { key: "data:b c.pack", file: "b c.pack", size: 20, sha256: H("b") },
    ]);
    const v1 = "# old\nws:1/a.pack\t# a.pack\ndata:x.pack\nnot a key\n";
    expect(parseExport(v1)).toEqual([
      { key: "ws:1/a.pack", file: "a.pack", size: null, sha256: null },
      { key: "data:x.pack", file: "x.pack", size: null, sha256: null },
    ]);
  });
});

describe("sync check", () => {
  const mods = [mod("ws:1/a.pack", "a.pack", 10, "workshop"), mod("ext:z:/dev/b.pack", "b.pack", 20, "folder"), mod("data:c.pack", "c.pack", 30)];
  const rows = parseExport(
    buildExport("MP", [
      { key: "ws:1/a.pack", file: "a.pack", size: 10, sha256: H("a") },
      { key: "data:b.pack", file: "b.pack", size: 20, sha256: H("b") },
    ]),
  );

  it("passes when everything matches (data row matched by file name in a folder)", () => {
    const r = verifyExport(rows, mods, ["ws:1/a.pack", "ext:z:/dev/b.pack"], { "ws:1/a.pack": H("a"), "ext:z:/dev/b.pack": H("b") });
    expect(r.rows.map((x) => x.status)).toEqual(["match", "match"]);
    expect(r.ok).toBe(true);
  });

  it("flags different files, disabled, missing, extras and order", () => {
    const r = verifyExport(rows, mods, ["ext:z:/dev/b.pack", "ws:1/a.pack", "data:c.pack"], { "ws:1/a.pack": H("f"), "ext:z:/dev/b.pack": H("b") });
    expect(r.rows.map((x) => x.status)).toEqual(["different", "match"]);
    expect(r.extra.map((m) => m.file)).toEqual(["c.pack"]);
    expect(r.orderDiffers).toBe(true);
    expect(r.ok).toBe(false);

    const disabled = verifyExport(rows, mods, ["ws:1/a.pack"], { "ws:1/a.pack": H("a") });
    expect(disabled.rows[1].status).toBe("disabled");

    const missing = verifyExport(parseExport("ws:9/zz.pack\tzz.pack\t1\t" + H("0")), mods, [], {});
    expect(missing.rows[0].status).toBe("missing");
  });

  it("size mismatch is decisive without hashing", () => {
    const r = verifyExport(parseExport("data:c.pack\tc.pack\t31\t" + H("c")), mods, ["data:c.pack"], {});
    expect(r.rows[0].status).toBe("different");
  });
});
