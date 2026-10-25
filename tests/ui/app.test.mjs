// Oberflächentest: startet die echte App (Rust-Server) und bedient sie in jsdom wie ein Nutzer.
import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, existsSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { JSDOM } from "jsdom";

const hier = dirname(fileURLToPath(import.meta.url));
const wurzel = join(hier, "..", "..");
const python = process.env.PYTHON || "python3";
let proc, adresse, dom, win, doc, fotoOrdner;

const warte = (ms) => new Promise((r) => setTimeout(r, ms));
async function warteBis(fn, text, ms = 15000) {
  const ende = Date.now() + ms;
  while (Date.now() < ende) {
    try { const v = fn(); if (v) return v; } catch { /* weiter */ }
    await warte(50);
  }
  throw new Error("Zeitüberschreitung: " + text + "\nSeite:\n" + doc.body.textContent.slice(0, 600));
}
const q = (s) => doc.querySelector(s);
const qa = (s) => [...doc.querySelectorAll(s)];
const klick = (el) => el.dispatchEvent(new win.MouseEvent("click", { bubbles: true }));
const taste = (key) => doc.dispatchEvent(new win.KeyboardEvent("keydown", { key, bubbles: true }));
function setze(el, wert) {
  el.value = wert;
  el.dispatchEvent(new win.Event("change", { bubbles: true }));
}
const buttonMitText = (t) => qa("button").find((b) => b.textContent.includes(t));

before(async () => {
  const basis = mkdtempSync(join(tmpdir(), "fb-ui-"));
  fotoOrdner = join(basis, "Fotos");
  const r = spawnSync(python, [join(hier, "make_fixture.py"), fotoOrdner], { encoding: "utf8" });
  assert.equal(r.status, 0, r.stderr);
  const bin = process.env.FOTOBUCH_BIN || join(wurzel, "target", "release", "fotobuch-dev");
  proc = spawn(bin, ["serve"], { cwd: wurzel });
  adresse = await new Promise((res, rej) => {
    let puffer = "";
    proc.stdout.on("data", (d) => {
      puffer += d;
      const m = puffer.match(/http:\/\/127\.0\.0\.1:\d+\//);
      if (m) res(m[0]);
    });
    proc.on("exit", () => rej(new Error("App beendet: " + puffer)));
    setTimeout(() => rej(new Error("Start dauert zu lange")), 15000);
  });
  dom = await JSDOM.fromURL(adresse, {
    runScripts: "dangerously",
    resources: "usable",
    pretendToBeVisual: true,
    beforeParse(w) { w.fetch = fetch; w.scrollTo = () => {}; },
  });
  win = dom.window;
  doc = win.document;
  win.addEventListener("error", (e) => { throw e.error; });
});

after(() => { proc && proc.kill(); win && win.close(); });

test("Start: Schritt 1 zeigt Hilfe, Datenschutz und gesperrte Folgeschritte", async () => {
  await warteBis(() => q("h1"), "Überschrift");
  assert.match(q("h1").textContent, /Fotos laden/);
  assert.ok(q(".privat").textContent.includes("lokal"));
  assert.ok(q("details.hilfe"), "Hilfetext vorhanden");
  const nav = qa("#schritte button");
  assert.equal(nav.length, 6);
  assert.ok(nav[1].disabled && nav[5].disabled, "Folgeschritte gesperrt");
  assert.equal(nav[0].getAttribute("aria-current"), "step");
  assert.ok(q("#buch").hidden, "Buch-Leiste erst nach Analyse");
});

test("Ordner öffnen mit falschem Pfad zeigt verständliche Meldung", async () => {
  setze(q("#pfad"), join(fotoOrdner, "gibtsnicht"));
  klick(q("#btn-oeffnen"));
  await warteBis(() => !q("#meldung").hidden, "Fehlermeldung");
  assert.match(q("#meldung").textContent, /existiert nicht/);
});

test("Ordner öffnen, analysieren, landet bei den Ereignissen", async () => {
  q("#pfad").value = fotoOrdner;
  klick(q("#btn-oeffnen"));
  await warteBis(() => q("#btn-analyse"), "Analyse-Knopf");
  assert.ok(q("#meldung").hidden);
  klick(q("#btn-analyse"));
  await warteBis(() => /Ereignisse/.test(q("h1")?.textContent) , "Schritt Ereignisse");
  assert.equal(qa(".ereignis").length, 2);
  assert.equal(q("#schritte [aria-current]").dataset.schritt, "2");
  assert.ok(!q("#buch").hidden);
  assert.equal(q("#buch-fotos").textContent, "19");
  assert.equal(q("#buch-seiten").textContent, "24");
  assert.match(q("#orte-info").textContent, /18 von 19|18 von 20|Orte/);
  assert.ok(qa(".ortchip").length >= 2, "Orte je Ereignis sichtbar");
});

test("Ereignis umbenennen wird gespeichert und in der Buch-Leiste gezeigt", async () => {
  const feld = q('input[data-key]');
  setze(feld, "Toskana Tag 1");
  await warteBis(() => q("#buch .liste").textContent.includes("Toskana Tag 1"), "Name in Leiste");
  assert.equal(q('input[data-key]').value, "Toskana Tag 1");
});

test("Aussortieren: Serie zeigt Vorschlag, Klick und Tastatur ändern die Auswahl", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#serie"), "Serienansicht");
  assert.match(q("#serie-titel").textContent, /Serie 1 von 1/);
  let karten = qa("#serie .karte");
  assert.equal(karten.length, 2);
  const vorschlag = karten.find((k) => k.querySelector(".marke-v"));
  assert.ok(vorschlag.classList.contains("behalten"));
  assert.ok(karten.find((k) => !k.querySelector(".marke-v")).classList.contains("weg"));
  // Zweites Foto per Leertaste dazunehmen
  taste("ArrowRight"); await warteBis(() => qa("#serie .karte")[1].classList.contains("fokus"), "Fokus");
  taste(" ");
  await warteBis(() => qa("#serie .karte").every((k) => k.classList.contains("behalten")), "beide behalten");
  assert.equal(q("#buch-fotos").textContent, "20");
  // Zurück zum Vorschlag mit V
  taste("v");
  await warteBis(() => qa("#serie .karte").filter((k) => k.classList.contains("behalten")).length === 1, "Vorschlag");
  assert.equal(q("#buch-fotos").textContent, "19");
  // Aussortierte mit Begründung sichtbar, Detailansicht holt zurück
  const weg = qa("#aussortiert .karte");
  assert.ok(weg.length >= 2, "Duplikat und Serienrest sind aufgelistet");
  const dup = weg.find((k) => /Exaktes Duplikat/.test(k.textContent));
  assert.ok(dup, "Duplikat mit Begründung");
  assert.match(dup.textContent, /byte-genau gleich/);
  klick(dup);
  await warteBis(() => !q("#lupe").hidden && q("#lupe-grund"), "Detailansicht");
  assert.match(q("#lupe-grund").textContent, /Nicht im Buch/);
  klick(q("#lupe-umschalten"));
  await warteBis(() => q("#buch-fotos").textContent === "20", "Duplikat zurückgeholt");
  assert.ok(q("#btn-reset"));
  klick(q("#btn-reset"));
  await warteBis(() => q("#buch-fotos").textContent === "19", "zurückgesetzt");
});

test("Orte: Rangliste, Benennen und Begrenzung pro Ort", async () => {
  await warteBis(() => qa(".ort").length === 2, "zwei Orte");
  const gross = qa(".ort").find((o) => o.querySelector(".ort-raster").children.length === 8);
  assert.ok(gross, "der Ort mit 12 Fotos zeigt die 8 besten");
  assert.equal(gross.querySelector(".marke-b").textContent, "#1");
  setze(q("input[data-ortkey]"), "Times Square");
  await warteBis(() => qa("input[data-ortkey]").some((i) => i.value === "Times Square"), "Ortsname gespeichert");
  // Begrenzung auf 3 pro Ort: 12 -> 3 und 6 -> 3, das Foto ohne GPS (s1) bleibt: 7
  assert.equal(q("#buch-fotos").textContent, "19");
  setze(q("#max-ort"), "3");
  await warteBis(() => q("#buch-fotos").textContent === "7", "Begrenzung wirkt");
  assert.ok(qa(".ort .karte.weg").length > 0, "entfernte Fotos sind markiert");
  // Ein entferntes Foto per Klick zurückholen
  klick(qa(".ort .karte.weg")[0]);
  await warteBis(() => q("#buch-fotos").textContent === "8", "Foto zurückgeholt");
  setze(q("#max-ort"), "");
  await warteBis(() => q("#buch-fotos").textContent === "19", "Begrenzung aus");
  klick(q("#btn-reset"));
  await warteBis(() => q("#buch-fotos").textContent === "19", "zurückgesetzt");
});

test("Seiten: Zähler, Profilwarnung, Kürzen-Vorschlag", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#zaehler"), "Zähler");
  assert.match(q("#zaehler").textContent, /24 Seiten/);
  assert.match(q("#zaehler").textContent, /fehlen/);
  // Eigene Seitenregel: ab 4 Seiten. 19 Fotos in 2 Ereignissen brauchen 7 Seiten, Schritt 2 -> 8
  setze(q("#seiten-min"), "4");
  await warteBis(() => q("#zaehler .gross")?.textContent === "8 Seiten", "8 Seiten");
  assert.equal(q("#seiten-min").value, "4");
  assert.equal(qa("#tabelle tbody tr").length, 2);
  assert.equal(q("#buch-seiten").textContent, "8");
});

test("Buch: Seiten mit Layout, Blättern, Übersicht, Detailansicht", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#doppelseite"), "Doppelseite");
  assert.ok(q("#btn-ansicht-doppel"));
  assert.match(q("#buch-pos").textContent, /Seite 1 von \d+/);
  assert.equal(qa("#doppelseite .seite").length, 1, "Seite 1 steht allein");
  const fotos = qa("#doppelseite .sfoto");
  assert.ok(fotos.length >= 3, "Fotos auf den Seiten");
  assert.ok(fotos.every((f) => /left:[\d.]+%/.test(f.getAttribute("style"))), "Positionen gesetzt");
  const vor = q("#buch-pos").textContent;
  klick(q("#btn-buch-weiter"));
  await warteBis(() => q("#buch-pos").textContent !== vor, "geblättert");
  assert.match(q("#buch-pos").textContent, /Seiten 2–3 von/);
  assert.equal(qa("#doppelseite .seite").length, 2, "danach Doppelseiten");
  // Anderes Layout für eine Seite wählen
  const vorher = q("#doppelseite .seite").innerHTML;
  klick(q(".layout-knopf"));
  await warteBis(() => q("#doppelseite .seite").innerHTML !== vorher, "anderes Layout");
  assert.match(q(".layout-knopf").textContent, /2\//);
  taste("ArrowLeft");
  await warteBis(() => q("#buch-pos").textContent === vor, "mit Pfeiltaste zurück");
  // Detailansicht mit Begründung
  klick(q("#doppelseite .sfoto"));
  await warteBis(() => !q("#lupe").hidden && q("#lupe-grund"), "Detail");
  assert.match(q("#lupe-grund").textContent, /Im Buch/);
  klick(q("#lupe-zu"));
  // Seitenformat ändern
  setze(q("#seitenformat"), "quadrat");
  await warteBis(() => q(".seite")?.getAttribute("style").includes("aspect-ratio:1"), "Quadratformat");
  // Übersicht
  klick(q("#btn-ansicht-uebersicht"));
  await warteBis(() => q("#uebersicht"), "Übersicht");
  assert.ok(qa("#uebersicht .seitenkarte").length >= 6);
  klick(qa("#uebersicht .seitenkarte")[2]);
  await warteBis(() => q("#doppelseite") && /Seiten 2–3/.test(q("#buch-pos").textContent), "Sprung zur Doppelseite");
});

test("Export: schreibt Dateien, zeigt Ergebnis, App lässt sich beenden", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#btn-export"), "Export-Knopf");
  const ziel = join(dirname(fotoOrdner), "Ausgabe");
  q("#ziel").value = ziel;
  klick(q("#btn-export"));
  await warteBis(() => q("#export-ergebnis"), "Export-Ergebnis");
  assert.match(q("#export-ergebnis").textContent, /19 Fotos kopiert/);
  assert.ok(existsSync(join(ziel, "seitenliste.csv")));
  assert.match(readFileSync(join(ziel, "LIESMICH.txt"), "utf8"), /Originalfotos wurden nicht verändert/);
  klick(q("#btn-beenden"));
  await warteBis(() => /Beendet/.test(q("h1").textContent), "Beendet-Seite");
});
