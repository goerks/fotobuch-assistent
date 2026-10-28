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
  throw new Error("Zeitüberschreitung: " + text + "\nSeite:\n" + (doc.querySelector(".ereigniskarte") || doc.body).textContent.slice(0, 500));
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
  assert.equal(nav.length, 7);
  assert.ok(nav[1].disabled && nav[6].disabled, "Folgeschritte gesperrt");
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

test("Ereignisse: zusammenlegen, wieder teilen, zurücksetzen", async () => {
  assert.equal(qa(".ereignis").length, 2);
  klick(q('button[data-aktion="zusammenlegen"]'));
  await warteBis(() => qa(".ereignis").length === 1, "zusammengelegt");
  assert.ok(q("#ereignisse-zurueck"), "Zurücksetzen sichtbar");
  klick(q('button[data-aktion="teilen"]'));
  await warteBis(() => q(".teilen-foto"), "Teilen-Panel");
  const fotos = qa(".teilen-foto");
  klick(fotos[Math.min(fotos.length - 1, 8)]);
  await warteBis(() => qa(".ereignis").length === 2, "geteilt");
  klick(q("#ereignisse-zurueck"));
  await warteBis(() => !q("#ereignisse-zurueck"), "zurückgesetzt");
  assert.equal(qa(".ereignis").length, 2);
});

test("Ereignisse: Feinheit-Regler und Zeitleiste mit Teilen und Zusammenlegen", async () => {
  const S = () => win.__fotobuch.S.a;
  assert.ok(q("#feinheit"), "Regler Feinheit");
  assert.match(q("#fein-zahl").textContent, /^2 Ereignisse$/);
  assert.ok(q("#luecke"), "genaue Einstellung bleibt unter «Genauer einstellen»");
  assert.ok(q("#zeitleiste") && qa("#zeitleiste .zl-bar").length >= 2, "Zeitleiste mit Balken");
  assert.ok(qa("#zeitleiste .zl-tag").length >= 1);
  // fein: Pause 0,5 Stunden
  setze(q("#feinheit"), "100");
  await warteBis(() => S().einstellungen.luecke_stunden === 0.5, "feine Einteilung gespeichert");
  const fein = S().zahlen.ereignisse;
  setze(q("#feinheit"), "0");
  await warteBis(() => S().einstellungen.luecke_stunden === 24, "grobe Einteilung gespeichert");
  assert.ok(S().zahlen.ereignisse <= 2 && S().zahlen.ereignisse <= fein, "grob ergibt nicht mehr Ereignisse als fein");
  setze(q("#luecke"), "6");
  await warteBis(() => S().einstellungen.luecke_stunden === 6 && S().zahlen.ereignisse === 2, "wieder 6 Stunden");
  // Zeitleiste: Klick auf die Grenze legt zusammen, Zurücksetzen stellt wieder her
  const grenze = q("#zeitleiste .zl-grenze");
  assert.ok(grenze, "Grenze zwischen den Ereignissen");
  klick(grenze);
  await warteBis(() => S().zahlen.ereignisse === 1, "über die Zeitleiste zusammengelegt");
  klick(q("#ereignisse-zurueck"));
  await warteBis(() => S().zahlen.ereignisse === 2, "zurückgesetzt");
  // lesbare Namen
  assert.match(q('input[data-key]').placeholder, /^(Mo|Di|Mi|Do|Fr|Sa|So) \d\d\.\d\d\. /);
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

test("Umfang: Bereich, Dichte, Seitenzahl je Ereignis und Auswahl-Galerie", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#zaehler"), "Zähler");
  assert.match(q("#zaehler").textContent, /24 Seiten/);
  assert.equal(qa(".ereigniskarte").length, 2, "eine Karte je Ereignis");
  assert.equal(q("#von").value, "2");
  assert.equal(q("#bis").value, "5");
  const plan = () => win.__fotobuch.S.a.seitenplan;
  const karte = (i) => qa(".ereigniskarte")[i];
  const seiten = (i) => Number(karte(i).querySelector(".seitenzahl").textContent.split(" ")[0]);
  // Das Ereignis mit wenigen Fotos hat eine Seite, kein Ereignis wird auf eine feste Zahl gezwungen
  const summe = () => plan().ereignisse.reduce((s, e) => s + e.seiten, 0);
  assert.equal(summe(), plan().seiten_benoetigt);
  assert.ok(plan().ereignisse.every((e) => e.seiten >= 1 && e.fotos / e.seiten <= 5 + 1e-9), "höchstens fünf Fotos pro Seite");

  // Eigene Seitenregel: ab 4 Seiten, die Zähler-Zahl folgt dem Profil
  setze(q("#seiten-min"), "4");
  await warteBis(() => win.__fotobuch.S.a.einstellungen.seiten_min === 4 && q("#zaehler .gross").textContent === plan().buch_seiten + " Seiten", "Seitenregel");
  assert.equal(plan().buch_seiten % 2, 0);

  // Seitenzahl eines Ereignisses von Hand ändern, auch ungerade
  const vorher = seiten(0);
  klick(karte(0).querySelector(".plus"));
  await warteBis(() => seiten(0) === vorher + 1, "eine Seite mehr");
  assert.match(karte(0).textContent, /von dir festgelegt/);
  assert.equal(plan().ereignisse[0].manuell, true);
  klick(karte(0).querySelector("button.klein"));
  await warteBis(() => seiten(0) === vorher && !/von dir festgelegt/.test(karte(0).textContent), "wieder automatisch");

  // Dichte
  klick(karte(1).querySelector('[data-dichte="1"]'));
  await warteBis(() => karte(1).querySelector('[data-dichte="1"]').getAttribute("aria-pressed") === "true", "Dichte gross");
  assert.ok(plan().ereignisse[1].seiten >= 1);
  klick(q("#umfang-zurueck"));
  await warteBis(() => karte(1).querySelector('[data-dichte="0"]').getAttribute("aria-pressed") === "true", "Dichte zurück");

  // Galerie: alle Fotos des Ereignisses, im Buch mit Seitenzahl, Klick überschreibt
  const galerie = karte(0).querySelector("details.galerie");
  assert.ok(galerie.open, "das erste Ereignis ist aufgeklappt");
  const ev0 = win.__fotobuch.S.a.ereignisse[0];
  assert.equal(galerie.querySelectorAll(".gfoto").length, ev0.anzahl);
  const drin = galerie.querySelector(".gfoto.drin");
  assert.match(drin.querySelector(".gmarke").textContent, /^S\. \d+$/, "Seitenzahl am Foto");
  const imBuch = Number(q("#buch-fotos").textContent);
  klick(drin);
  await warteBis(() => Number(q("#buch-fotos").textContent) === imBuch - 1, "Foto herausgenommen");
  const id = drin.dataset.id;
  const nach = () => qa(".gfoto").find((x) => x.dataset.id === id);
  assert.ok(nach().classList.contains("draussen") && nach().classList.contains("eigen"), "als eigene Wahl markiert");
  klick(nach());
  await warteBis(() => Number(q("#buch-fotos").textContent) === imBuch, "Foto zurückgeholt");
});

test("Gestaltung: Thema wählen, anpassen, Deckblatt und Titelfoto", async () => {
  klick(q("#btn-naechster"));
  await warteBis(() => q("#themen"), "Themen");
  assert.equal(qa(".themakarte").length, 6);
  assert.equal(q(".themakarte.aktiv").dataset.thema, "klar", "Standard ist Klar");
  assert.ok(q("#gestaltvorschau .deckblatt"), "Deckblatt in der Vorschau");
  assert.ok(qa("#gestaltvorschau .sbeschr").length >= 1, "Unterschrift auf der ersten Seite");
  klick(q('.themakarte[data-thema="nacht"]'));
  await warteBis(() => q(".themakarte.aktiv")?.dataset.thema === "nacht", "Thema Nacht");
  assert.ok(q("#gestaltvorschau .seite").classList.contains("th-nacht"));
  assert.ok(/--t-seite:\s*#131417/.test(q("#gestaltvorschau .seite").getAttribute("style")));
  setze(q("#g-rahmen"), "polaroid");
  await warteBis(() => q("#gestaltvorschau .seite").classList.contains("rm-polaroid"), "Sofortbild-Rahmen");
  setze(q("#g-hintergrund"), "kraft");
  await warteBis(() => q("#gestaltvorschau .seite").classList.contains("hg-kraft"), "Kraftpapier");
  setze(q("#g-farbton"), "#aa3300");
  await warteBis(() => /--t-akzent:\s*#aa3300/.test(q("#gestaltvorschau .seite").getAttribute("style")), "eigener Farbton");
  assert.ok(q("#g-farbton-zurueck"), "Weg zurück zur Themenfarbe");
  setze(q("#g-titel"), "Unsere Reise");
  await warteBis(() => q("#gestaltvorschau .dk-titel").textContent === "Unsere Reise", "Titel");
  setze(q("#g-name"), "Familie Muster");
  await warteBis(() => q("#gestaltvorschau .dk-name").textContent === "Familie Muster", "Name");
  klick(qa("#titelfotos .titelfoto")[1]);
  await warteBis(() => qa("#titelfotos .titelfoto")[1]?.classList.contains("aktiv"), "Titelfoto gewählt");
  klick(q("#g-zurueck"));
  await warteBis(() => !q("#g-farbton-zurueck") && q("#g-rahmen").value === "thema" && q("#g-hintergrund").value === "thema", "auf das Thema zurückgesetzt");
  assert.equal(q("#g-titel").value, "Unsere Reise", "Texte bleiben");
  klick(q('.themakarte[data-thema="klar"]'));
  await warteBis(() => q(".themakarte.aktiv")?.dataset.thema === "klar", "zurück zu Klar");
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
  // Layout aus Vorlagen-Vorschau wählen
  const vorher = q("#doppelseite .seite").innerHTML;
  klick(q(".layout-knopf"));
  await warteBis(() => qa(".layoutwahl .layoutoption").length >= 2, "Vorlagen sichtbar");
  assert.ok(q(".layoutoption.aktiv"), "aktuelles Layout markiert");
  klick(qa(".layoutwahl .layoutoption")[1]);
  await warteBis(() => q("#doppelseite .seite").innerHTML !== vorher, "anderes Layout");
  await warteBis(() => qa(".layoutwahl .layoutoption")[1]?.classList.contains("aktiv"), "Auswahl markiert");
  klick(q("#layout-zu"));
  await warteBis(() => !q(".layoutwahl"), "Auswahl geschlossen");
  // Detailansicht mit Begründung
  klick(q("#doppelseite .sfoto"));
  await warteBis(() => !q("#lupe").hidden && q("#lupe-grund"), "Detail");
  assert.match(q("#lupe-grund").textContent, /Im Buch/);
  klick(q("#lupe-zu"));
  // Seitenformat ändern
  setze(q("#seitenformat"), "quadrat");
  await warteBis(() => /aspect-ratio:\s*1(;|$)/.test(q(".seite")?.getAttribute("style") || ""), "Quadratformat");
  // Übersicht
  klick(q("#btn-ansicht-uebersicht"));
  await warteBis(() => q("#uebersicht"), "Übersicht");
  assert.ok(qa("#uebersicht .seitenkarte").length >= 6);
  klick(qa("#uebersicht .seitenkarte")[2]);
  await warteBis(() => q("#doppelseite") && /Seiten 2–3/.test(q("#buch-pos").textContent), "Sprung zur Doppelseite");
});

test("Seitenband: Seite wählen, verschieben, leere Seite einfügen, entfernen, zurücknehmen", async () => {
  const seitenZahl = () => qa("#band .bandseite").length;
  const nrn = () => qa("#band .bandseite").map((b) => b.dataset.nr + ":" + b.dataset.key);
  await warteBis(() => q("#band") && seitenZahl() >= 3, "Seitenband");
  const n0 = seitenZahl();
  const key2 = qa("#band .bandseite")[1].dataset.key;
  const key1 = qa("#band .bandseite")[0].dataset.key;
  assert.ok(q("#band-vor").disabled, "ohne Wahl keine Aktionen");
  // wählen springt zur Doppelseite
  klick(qa("#band .bandseite")[1]);
  await warteBis(() => qa("#band .bandseite")[1].classList.contains("gewaehlt") && !q("#band-vor").disabled, "gewählt");
  // nach vorn: Seite 2 wird Seite 1
  klick(q("#band-vor"));
  await warteBis(() => qa("#band .bandseite")[0].dataset.key === key2, "Seite nach vorn");
  assert.equal(qa("#band .bandseite")[1].dataset.key, key1);
  // leere Seite danach
  klick(q("#band-leer-danach"));
  await warteBis(() => seitenZahl() === n0 + 1, "leere Seite eingefügt");
  assert.ok(qa("#band .bandseite")[1].classList.contains("leer"), "leere Seite steht hinter der gewählten");
  assert.ok(q("#seitenfolge-zurueck"));
  // leere Seite wählen und löschen
  klick(qa("#band .bandseite")[1]);
  await warteBis(() => /Leere Seite löschen/.test(q("#band-entfernen").textContent), "Löschen angeboten");
  klick(q("#band-entfernen"));
  await warteBis(() => seitenZahl() === n0 && !qa("#band .bandseite.leer").length, "leere Seite gelöscht");
  // normale Seite entfernen: eine Seite weniger, Fotos bleiben im Buch
  const fotos = Number(q("#buch-fotos").textContent);
  klick(qa("#band .bandseite")[2]);
  await warteBis(() => !q("#band-entfernen").disabled, "Seite 3 gewählt");
  klick(q("#band-entfernen"));
  await warteBis(() => seitenZahl() === n0 - 1, "Seite entfernt");
  assert.equal(Number(q("#buch-fotos").textContent), fotos);
  // alles zurücknehmen
  klick(q("#seitenfolge-zurueck"));
  await warteBis(() => seitenZahl() === n0 && qa("#band .bandseite")[0].dataset.key === key1, "Seitenfolge zurückgesetzt");
  assert.ok(nrn().length === n0);
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
