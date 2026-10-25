/* Fotobuch-Assistent: Oberfläche. Reines JavaScript, keine externen Ressourcen. */
(function () {
  "use strict";

  const TOKEN = (document.querySelector('meta[name="fotobuch-token"]') || {}).content || "";
  const SCHRITTE = [
    { nr: 1, name: "Fotos laden" },
    { nr: 2, name: "Ereignisse" },
    { nr: 3, name: "Aussortieren" },
    { nr: 4, name: "Seiten" },
    { nr: 5, name: "Buch" },
    { nr: 6, name: "Export" },
  ];
  const WOCHENTAG = ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"];

  const S = {
    status: null,
    a: null, // Ansicht vom Server
    schritt: 1,
    serie: 0, // Index der gezeigten Serie
    fokus: 0, // Fokus innerhalb der Serie
    zeigeEntfernt: 60,
    zeigeOrte: 12,
    teilenOffen: null, // Schlüssel des Ereignisses, das gerade geteilt wird
    kg: 280, // Bildgrösse in Schritt 3 (Pixel)
    wegFilter: "alle",
    buchPos: 0, // Doppelseite (0-basiert)
    buchAnsicht: "doppel",
    layoutOffen: null,
    zoom: 200,
    exportErgebnis: null,
    beendet: false,
    poll: null,
    busy: false,
  };

  // ------------------------------------------------------------------ Hilfen

  const $ = (s, r) => (r || document).querySelector(s);

  function h(tag, attrs, ...kids) {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (v === null || v === undefined || v === false) continue;
      if (k === "class") e.className = v;
      else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else if (k === "value") e.value = v;
      else if (k === "checked") e.checked = !!v;
      else e.setAttribute(k, v === true ? "" : v);
    }
    for (const c of kids.flat(2)) {
      if (c === null || c === undefined || c === false) continue;
      e.append(c.nodeType ? c : document.createTextNode(String(c)));
    }
    return e;
  }

  function url(pfad) {
    return location.origin + pfad;
  }

  async function api(pfad, body) {
    const opt =
      body === undefined
        ? { headers: { "X-Fotobuch-Token": TOKEN } }
        : {
            method: "POST",
            headers: { "X-Fotobuch-Token": TOKEN, "Content-Type": "application/json" },
            body: JSON.stringify(body),
          };
    let r;
    try {
      r = await fetch(url(pfad), opt);
    } catch (e) {
      throw new Error("Die App antwortet nicht. Wurde das Programmfenster geschlossen?");
    }
    let d = {};
    try {
      d = await r.json();
    } catch (e) {
      /* leer */
    }
    if (!r.ok) throw new Error(d.fehler || "Fehler " + r.status);
    return d;
  }

  function vorschauUrl(id, k) {
    return "/api/vorschau/" + id + "?k=" + (k || 240) + "&t=" + encodeURIComponent(TOKEN);
  }

  function meldung(text) {
    const m = $("#meldung");
    if (!text) {
      m.hidden = true;
      m.textContent = "";
    } else {
      m.hidden = false;
      m.textContent = text;
    }
  }

  // Aktionen laufen nacheinander. Eine zweite Eingabe während der ersten geht nicht verloren.
  let warteschlange = Promise.resolve();
  function aktion(fn) {
    const lauf = warteschlange.then(async () => {
      S.busy = true;
      meldung("");
      try {
        await fn();
      } catch (e) {
        meldung(e.message);
      } finally {
        S.busy = false;
      }
    });
    warteschlange = lauf;
    return lauf;
  }

  // Im App-Fenster öffnen externe Links im Standardbrowser (über den lokalen Server).
  document.addEventListener("click", (e) => {
    const a = e.target.closest && e.target.closest("a[href]");
    if (!a || !S.status || !S.status.nativ) return;
    if (/^https?:\/\//.test(a.getAttribute("href"))) {
      e.preventDefault();
      api("/api/extern", { url: a.href }).catch(() => {});
    }
  });

  async function lade() {
    S.status = await api("/api/status");
    if (S.status.hat_projekt) {
      S.a = await api("/api/ansicht");
    } else {
      S.a = null;
    }
  }

  async function ladeUndZeige() {
    await lade();
    zeige();
  }

  const pl = (n, ein, mehr) => n + " " + (n === 1 ? ein : mehr);
  const zahl = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, "’");

  function zeitTeil(iso) {
    // "2026-07-03T08:40"
    const d = new Date(iso.length === 16 ? iso + ":00" : iso);
    const p = (x) => String(x).padStart(2, "0");
    return {
      tag: WOCHENTAG[d.getDay()] + " " + p(d.getDate()) + "." + p(d.getMonth() + 1) + ".",
      zeit: p(d.getHours()) + ":" + p(d.getMinutes()),
      jahr: d.getFullYear(),
      ganz: d.getFullYear() + p(d.getMonth() + 1) + p(d.getDate()),
    };
  }

  function zeitraum(von, bis) {
    const a = zeitTeil(von);
    const b = zeitTeil(bis);
    if (a.ganz === b.ganz) return a.tag + " " + a.jahr + ", " + a.zeit + "–" + b.zeit;
    return a.tag + " " + a.zeit + " bis " + b.tag + " " + b.zeit + " (" + a.jahr + ")";
  }

  function ereignisName(e) {
    return e.name || e.titel;
  }

  function hilfe(titel, ...inhalt) {
    return h("details", { class: "hilfe" }, h("summary", {}, titel), ...inhalt);
  }

  // ------------------------------------------------------------------ Navigation

  function kannSchritt(nr) {
    if (nr === 1) return true;
    return !!(S.a && S.a.analysiert);
  }

  async function geheZu(nr) {
    if (!kannSchritt(nr)) return;
    S.schritt = nr;
    if (S.a && S.a.analysiert) {
      try {
        await api("/api/schritt", { schritt: nr });
      } catch (e) {
        /* nicht kritisch */
      }
    }
    zeige();
    const m = $("#inhalt");
    if (m && m.focus) m.focus({ preventScroll: true });
    window.scrollTo(0, 0);
  }

  function schrittZahl(nr) {
    const a = S.a;
    if (!a || !a.analysiert) return "";
    const z = a.zahlen;
    if (nr === 1) return zahl(z.dateien);
    if (nr === 2) return String(z.ereignisse);
    if (nr === 3) return zahl(z.im_buch);
    if (nr === 4 || nr === 5) return a.seitenplan.buch_seiten + " S.";
    return "";
  }

  function zeigeNavigation() {
    const nav = $("#schritte");
    nav.replaceChildren(
      ...SCHRITTE.map((s) =>
        h(
          "button",
          {
            type: "button",
            "data-schritt": s.nr,
            "aria-current": S.schritt === s.nr ? "step" : null,
            class: S.a && S.a.analysiert && s.nr < S.schritt ? "fertig" : "",
            disabled: !kannSchritt(s.nr),
            onclick: () => geheZu(s.nr),
          },
          h("span", { class: "nr" }, S.a && S.a.analysiert && s.nr < S.schritt ? "✓" : s.nr),
          h("span", { class: "name" }, s.name),
          h("span", { class: "zahl" }, schrittZahl(s.nr))
        )
      )
    );
  }

  // ------------------------------------------------------------------ Seitenleiste «Dein Buch»

  function zeigeBuch() {
    const side = $("#buch");
    if (!S.a || !S.a.analysiert || S.schritt === 5) {
      side.hidden = true;
      return;
    }
    side.hidden = false;
    const p = S.a.seitenplan;
    const z = S.a.zahlen;
    let status;
    if (p.ueber_maximum) {
      status = h("div", { class: "warn" }, "Mehr Seiten als das Profil erlaubt (" + S.a.profil.seiten_max + ").");
    } else if (p.fehlende_fotos === 0) {
      status = h("div", { class: "gut" }, "Alle Seiten sind gefüllt.");
    } else {
      status = h("p", { class: "still" }, "Bis zur vollen Seitenzahl fehlen noch ", h("b", {}, p.fehlende_fotos), " Fotos.");
    }
    side.replaceChildren(
      h("h2", {}, "Dein Buch"),
      h("div", { class: "wert", id: "buch-fotos" }, zahl(z.im_buch)),
      h("div", { class: "still" }, "Fotos im Buch (von " + zahl(z.dateien) + ")"),
      h("div", { class: "wert", id: "buch-seiten", style: "margin-top:12px" }, p.buch_seiten),
      h("div", { class: "still" }, "Seiten · " + p.seiten_benoetigt + " nötig bei " + p.fotos_pro_seite + " Fotos pro Seite"),
      status,
      h(
        "ul",
        { class: "liste" },
        S.a.ereignisse.map((e) =>
          h("li", {}, h("span", {}, ereignisName(e)), h("span", { class: "still" }, e.im_buch + " F · " + e.seiten + " S"))
        )
      )
    );
  }

  // ------------------------------------------------------------------ Schritt 1: Fotos laden

  function viewStart() {
    const st = S.status;
    const job = st.job;
    const v = h("div", {});
    v.append(
      h("h1", {}, "Fotos laden"),
      h(
        "p",
        { class: "still" },
        "Wähle den Ordner mit deinen Fotos. Der Assistent liest sie nur und verändert nie ein Original. Alles bleibt auf diesem Rechner."
      ),
      hilfe(
        "Was passiert hier?",
        h("p", {}, "Der Assistent sucht im Ordner und allen Unterordnern nach JPG, HEIC, PNG und RAW-Dateien. Er liest Aufnahmezeit, Ort, Grösse und Schärfe und merkt sich das im Unterordner «.fotobuch». Beim zweiten Mal geht es deshalb viel schneller."),
        h("p", {}, "Fotos ohne Aufnahmezeit kommen als eigenes Ereignis ans Ende des Buchs, nach Dateiname geordnet..")
      )
    );

    if (job.laeuft) {
      const proz = job.gesamt ? Math.round((100 * job.fertig) / job.gesamt) : 0;
      v.append(
        h("h2", {}, "Analyse läuft …"),
        h("div", { class: "balken", role: "progressbar", "aria-valuenow": proz, "aria-valuemin": 0, "aria-valuemax": 100 }, h("div", { style: "width:" + proz + "%" })),
        h("p", { class: "still", id: "fortschritt-text" }, job.gesamt ? job.fertig + " von " + job.gesamt + " neuen oder geänderten Fotos" : "Ordner wird durchsucht …")
      );
      return v;
    }
    if (job.fehler) v.append(h("div", { class: "warn" }, "Die Analyse ist fehlgeschlagen: " + job.fehler));

    const eingabe = h("input", { type: "text", id: "pfad", class: "breit", placeholder: "z. B. C:\\Users\\Name\\Bilder\\Italien 2026", value: st.ordner || "", "aria-label": "Pfad zum Fotoordner" });
    const oeffnen = () =>
      aktion(async () => {
        await api("/api/ordner/oeffnen", { pfad: eingabe.value });
        await ladeUndZeige();
      });
    eingabe.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter") oeffnen();
    });
    v.append(
      h("h2", {}, "1. Ordner wählen"),
      h(
        "div",
        { class: "zeile" },
        st.kann_dialog
          ? h(
              "button",
              {
                type: "button",
                class: "knopf haupt",
                id: "btn-waehlen",
                onclick: () =>
                  aktion(async () => {
                    await api("/api/ordner/waehlen", {});
                    await ladeUndZeige();
                  }),
              },
              "Ordner wählen …"
            )
          : null,
        h("span", { class: "still" }, st.kann_dialog ? "oder Pfad eingeben:" : "Pfad eingeben:")
      ),
      h("div", { class: "zeile" }, eingabe, h("button", { type: "button", class: "knopf", id: "btn-oeffnen", onclick: oeffnen }, "Ordner öffnen"))
    );

    if (st.hat_projekt) {
      const a = S.a;
      v.append(h("h2", {}, "2. Fotos einlesen"), h("p", {}, "Ordner: ", h("b", {}, st.ordner)));
      if (a && a.analysiert) {
        v.append(
          h("div", { class: "gut" }, zahl(a.zahlen.dateien) + " Fotos gefunden" + (a.analyse_dauer != null ? " (letzte Analyse: " + a.analyse_dauer + " s)" : "") + ". Du kannst direkt weitermachen."),
          h("div", { class: "zeile" },
            h("button", { type: "button", class: "knopf haupt", id: "btn-weiter", onclick: () => geheZu(Math.max(2, Math.min(a.schritt, 5))) }, "Weiter mit meinem Buch"),
            h("button", { type: "button", class: "knopf", id: "btn-analyse", onclick: starteAnalyse }, "Ordner neu einlesen"))
        );
        if (a.zahlen.nicht_lesbar) v.append(h("div", { class: "warn" }, a.zahlen.nicht_lesbar + " Dateien konnten nicht gelesen werden. Sie werden nicht ins Buch übernommen."));
      } else {
        if (a && a.leer) v.append(h("div", { class: "warn" }, "In diesem Ordner wurden keine Fotos gefunden (JPG, HEIC, PNG, RAW). Prüfe den Ordner oder wähle einen anderen."));
        v.append(h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf haupt", id: "btn-analyse", onclick: starteAnalyse }, "Fotos analysieren")));
      }
    }
    return v;
  }

  function starteAnalyse() {
    return aktion(async () => {
      await api("/api/analyse/start", {});
      await lade();
      zeige();
      pollStart();
    });
  }

  // Wartet auf das Ende des Seitenexports und zeigt dabei den Fortschritt.
  async function seitenPoll() {
    for (;;) {
      S.status = await api("/api/status");
      zeige();
      if (!S.status.job.laeuft) return;
      await new Promise((r) => setTimeout(r, 400));
    }
  }

  function pollStart() {
    if (S.poll) return;
    S.poll = setInterval(async () => {
      try {
        const st = await api("/api/status");
        S.status = st;
        if (!st.job.laeuft) {
          clearInterval(S.poll);
          S.poll = null;
          await lade();
          if (S.a && S.a.analysiert && !st.job.fehler) S.schritt = 2;
          zeige();
        } else {
          const t = $("#fortschritt-text");
          const b = $(".balken > div");
          const j = st.job;
          if (t) t.textContent = j.gesamt ? j.fertig + " von " + j.gesamt + " neuen oder geänderten Fotos" : "Ordner wird durchsucht …";
          if (b) b.style.width = (j.gesamt ? Math.round((100 * j.fertig) / j.gesamt) : 0) + "%";
        }
      } catch (e) {
        meldung(e.message);
        clearInterval(S.poll);
        S.poll = null;
      }
    }, 500);
  }

  // ------------------------------------------------------------------ Schritt 2: Ereignisse

  function viewEreignisse() {
    const a = S.a;
    const z = a.zahlen;
    const v = h("div", {});
    v.append(
      h("h1", {}, "Ereignisse"),
      h("p", { class: "still" }, "Der Assistent hat deine Fotos nach der Aufnahmezeit sortiert und in " + z.ereignisse + " Ereignisse geteilt (Tage, Ausflüge, Anlässe). Gib ihnen Namen, sie erscheinen später im Export."),
      hilfe(
        "Wie wird geteilt?",
        h("p", {}, "Eine neue Gruppe beginnt, wenn zwischen zwei Fotos mehr als die eingestellte Zeitlücke liegt oder der Ort mehr als " + a.einstellungen.max_km + " km springt. Ist die Einteilung zu fein oder zu grob, ändere die Zeitlücke."),
        h("p", {}, "Beispiele: 6 Stunden trennt Vormittag und Abend am selben Tag nicht, aber die Nacht dazwischen. 2 Stunden macht aus einem Tag mehrere Gruppen.")
      )
    );
    if (z.ohne_aufnahmezeit) {
      v.append(h("div", { class: "warn" }, z.ohne_aufnahmezeit + " Fotos haben keine Aufnahmezeit. Sie stehen als eigenes Ereignis am Schluss."));
    }
    const luecke = h("input", { type: "number", id: "luecke", min: 0.5, max: 72, step: 0.5, value: a.einstellungen.luecke_stunden, "aria-label": "Zeitlücke in Stunden" });
    luecke.addEventListener("change", () =>
      aktion(async () => {
        await api("/api/einstellungen", { luecke_stunden: Number(luecke.value) });
        await ladeUndZeige();
      })
    );
    v.append(h("div", { class: "zeile" }, h("label", { for: "luecke" }, "Neues Ereignis nach einer Pause von"), luecke, h("span", {}, "Stunden")));
    const zo = a.zahlen_orte;
    if (zo.mit_gps) {
      const radius = h("input", { type: "number", id: "ort-radius", min: 25, max: 5000, step: 25, value: a.einstellungen.ort_radius_m, "aria-label": "Ortsradius in Metern" });
      radius.addEventListener("change", () =>
        aktion(async () => {
          await api("/api/einstellungen", { ort_radius_m: Number(radius.value) });
          await ladeUndZeige();
        })
      );
      v.append(
        h("div", { class: "zeile" }, h("label", { for: "ort-radius" }, "Fotos gelten als gleicher Ort bis"), radius, h("span", {}, "Meter Umkreis")),
        h("p", { class: "still", id: "orte-info" }, zo.mit_gps + " von " + a.zahlen.lesbar + " Fotos haben GPS-Daten und ergeben " + pl(zo.orte, "Ort", "Orte") + ". Fotos ohne GPS bekommen den Ort des zeitlich nächsten Fotos (bis 30 Minuten). Die Orte prüfst und begrenzt du im nächsten Schritt.")
      );
    } else {
      v.append(h("p", { class: "still", id: "orte-info" }, "Keines der Fotos hat GPS-Daten. Deshalb gibt es keine Orte. Das ist normal bei Kameras ohne Ortung oder wenn Ortungsdienste aus waren."));
    }

    for (const e of a.ereignisse) {
      const name = h("input", { type: "text", class: "breit", "data-key": e.key, value: e.name, placeholder: e.titel, "aria-label": "Name von Ereignis " + e.nummer, maxlength: 80 });
      name.addEventListener("change", () =>
        aktion(async () => {
          await api("/api/ereignis/name", { key: e.key, name: name.value });
          await ladeUndZeige();
        })
      );
      v.append(
        h(
          "div",
          { class: "ereignis" },
          h("div", {}, name, h("div", { class: "zeit" }, e.ohne_datum ? "Diese Fotos enthalten keine Aufnahmezeit. Sie stehen am Schluss des Buchs, nach Dateiname." : zeitraum(e.von, e.bis) + (e.ohne_zeit ? " · " + e.ohne_zeit + " ohne Aufnahmezeit" : ""))),
          h("div", { class: "zahl" }, h("b", {}, e.im_buch), " von " + e.anzahl + " Fotos", h("br"), e.seiten + (e.seiten === 1 ? " Seite" : " Seiten")),
          e.orte.length ? h("div", { class: "orte-zeile" }, "Orte: ", e.orte.slice(0, 6).map((o) => h("span", { class: "ortchip" }, o[1] + " · " + o[2])), e.orte.length > 6 ? " und " + (e.orte.length - 6) + " weitere" : null) : null,
          h("div", { class: "streifen" }, e.vorschau.map((id) => h("img", { src: vorschauUrl(id, 160), alt: "", loading: "lazy" }))),
          h(
            "div",
            { class: "ev-werkzeuge" },
            e.nummer > 1
              ? h("button", { class: "klein", "data-aktion": "zusammenlegen", onclick: () => aktion(async () => { await api("/api/ereignis/zusammenlegen", { key: e.key }); S.teilenOffen = null; await ladeUndZeige(); }) }, "Mit vorherigem zusammenlegen")
              : null,
            e.anzahl > 1
              ? h("button", { class: "klein", "data-aktion": "teilen", onclick: () => { S.teilenOffen = S.teilenOffen === e.key ? null : e.key; zeige(); } }, S.teilenOffen === e.key ? "Teilen abbrechen" : "Teilen …")
              : null
          ),
          S.teilenOffen === e.key
            ? h(
                "div",
                { class: "teilen-panel" },
                h("p", { class: "still" }, "Klicke das Foto, mit dem das neue Ereignis beginnen soll."),
                h(
                  "div",
                  { class: "teilen-raster" },
                  e.ids.map((id, k) =>
                    k === 0
                      ? h("div", { class: "teilen-start", title: "Erstes Foto des Ereignisses" }, h("img", { src: vorschauUrl(id, 160), alt: "" }))
                      : h("button", { class: "teilen-foto", "data-id": id, title: "Hier neu beginnen", onclick: () => aktion(async () => { await api("/api/ereignis/teilen", { id }); S.teilenOffen = null; await ladeUndZeige(); }) }, h("img", { src: vorschauUrl(id, 160), alt: "Neues Ereignis ab hier" }))
                  )
                )
              )
            : null
        )
      );
    }
    if (z.ereignis_aenderungen) {
      v.append(
        h("div", { class: "zeile" }, h("span", {}, z.ereignis_aenderungen + " manuelle Änderung(en) an der Einteilung. "), h("button", { class: "klein", id: "ereignisse-zurueck", onclick: () => aktion(async () => { await api("/api/ereignisse/zuruecksetzen", {}); S.teilenOffen = null; await ladeUndZeige(); }) }, "Automatische Einteilung wiederherstellen"))
      );
    }
    v.append(fuss(1, 3));
    return v;
  }

  function fuss(zurueck, weiter, weiterText) {
    return h(
      "div",
      { class: "fuss" },
      zurueck ? h("button", { type: "button", class: "knopf", id: "btn-zurueck", onclick: () => geheZu(zurueck) }, "← Zurück") : h("span"),
      weiter ? h("button", { type: "button", class: "knopf haupt", id: "btn-naechster", onclick: () => geheZu(weiter) }, weiterText || "Weiter →") : h("span")
    );
  }

  // ------------------------------------------------------------------ Schritt 3: Aussortieren

  function serienListe() {
    return S.a.serien;
  }

  function naechsteOffene(von) {
    const s = serienListe();
    for (let i = 1; i <= s.length; i++) {
      const j = (von + i) % s.length;
      if (!s[j].entschieden) return j;
    }
    return von;
  }

  function begrenzeSerie() {
    const n = serienListe().length;
    if (S.serie >= n) S.serie = Math.max(0, n - 1);
    if (S.serie < 0) S.serie = 0;
    const g = n ? serienListe()[S.serie] : null;
    if (g && S.fokus >= g.ids.length) S.fokus = 0;
  }

  function viewAussortieren() {
    const a = S.a;
    const z = a.zahlen;
    begrenzeSerie();
    const v = h("div", {});
    v.append(
      h("h1", {}, "Aussortieren"),
      h("p", { class: "still" }, "Doppelte, Screenshots und fast gleiche Fotos sind vorsortiert. Nichts wird gelöscht: Du entscheidest, was ins Buch kommt, und kannst jede Entscheidung ändern."),
      hilfe(
        "Wie arbeite ich hier?",
        h("ul", {},
          h("li", {}, "Exakte Duplikate (gleiche Datei) und Screenshots sind schon aussortiert. Unten kannst du sie zurückholen."),
          h("li", {}, "Eine Serie sind Fotos, die kurz nacheinander fast gleich aufgenommen wurden. Der Vorschlag ist das schärfste Bild mit der besten Auflösung."),
          h("li", {}, "Klicke ein Foto an, um es zu behalten oder zu entfernen. Es dürfen mehrere bleiben."),
          h("li", {}, "Mit «Alle Vorschläge übernehmen» gehst du schnell durch, wenn du dem Assistenten vertraust.")
        )
      ),
      h(
        "div",
        { class: "chips" },
        chip(z.exakte_gruppen, "Gruppen mit exakten Duplikaten"),
        chip(z.screenshots, "Screenshots"),
        chip(z.serien_entschieden + " / " + z.serien, "Serien geprüft"),
        chip(z.im_buch, "Fotos im Buch")
      )
    );
    const kg = h("input", { type: "range", id: "kg", min: 160, max: 520, step: 20, value: S.kg, "aria-label": "Bildgrösse" });
    kg.addEventListener("input", () => {
      S.kg = Number(kg.value);
      document.querySelectorAll(".serie").forEach((e) => e.style.setProperty("--kg", S.kg + "px"));
      document.querySelectorAll(".raster.gross").forEach((e) => e.style.setProperty("--kg", Math.round(S.kg * 0.85) + "px"));
    });
    v.append(h("div", { class: "zeile" }, h("label", { for: "kg" }, "Bildgrösse"), kg));

    // Serien
    v.append(h("h2", {}, "Fast gleiche Serien"));
    const serien = serienListe();
    if (!serien.length) {
      v.append(h("div", { class: "leer" }, "Keine Serien gefunden. Es gibt nichts zu prüfen."));
    } else {
      const g = serien[S.serie];
      const ev = a.ereignisse[g.ereignis];
      v.append(
        h(
          "div",
          { class: "zeile" },
          h("button", { type: "button", class: "knopf", id: "btn-serie-zurueck", onclick: () => springeSerie(S.serie - 1) }, "← Vorherige"),
          h("b", { id: "serie-titel" }, "Serie " + (S.serie + 1) + " von " + serien.length),
          h("span", { class: "still" }, (g.entschieden ? "geprüft" : "noch offen") + " · " + ereignisName(ev)),
          h("button", { type: "button", class: "knopf", id: "btn-serie-weiter", onclick: () => springeSerie(S.serie + 1) }, "Nächste →"),
          h("button", { type: "button", class: "knopf", id: "btn-serie-offen", onclick: () => springeSerie(naechsteOffene(S.serie)) }, "Nächste offene"),
          h("button", { type: "button", class: "knopf", id: "btn-alle-vorschlaege", onclick: alleVorschlaege }, "Alle Vorschläge übernehmen")
        )
      );
      v.append(
        h("div", { class: "serie", id: "serie", style: "--kg:" + S.kg + "px" },
          g.ids.map((id, i) => serienKarte(id, i, g)))
      );
      v.append(
        h("div", { class: "zeile" },
          h("button", { type: "button", class: "knopf", id: "btn-vorschlag", onclick: () => serieAktion(g.nr, "vorschlag") }, "Zurück zum Vorschlag"),
          h("button", { type: "button", class: "knopf", id: "btn-keines", onclick: () => serieAktion(g.nr, "keines") }, "Keines davon behalten")),
        h("div", { class: "tastatur" },
          "Tastatur: ", h("kbd", {}, "←"), " ", h("kbd", {}, "→"), " Foto wählen · ", h("kbd", {}, "Leertaste"), " behalten/entfernen · ",
          h("kbd", {}, "N"), " nächste · ", h("kbd", {}, "P"), " vorherige · ", h("kbd", {}, "V"), " Vorschlag · ", h("kbd", {}, "G"), " gross ansehen")
      );
    }

    v.append(...viewOrte(a));

    // Aussortierte mit Begründung
    const KAT = [
      ["alle", "Alle"], ["Exaktes Duplikat", "Exakte Duplikate"], ["Screenshot", "Screenshots"], ["Serie", "Aus Serien"],
      ["Ort", "Wegen Ortsbegrenzung"], ["Von dir", "Von dir entfernt"],
    ];
    const alleWeg = Object.entries(a.fotos).filter(([id, f]) => !f.b && !f.x);
    const zaehleKat = (k) => (k === "alle" ? alleWeg.length : alleWeg.filter(([, f]) => f.g.startsWith(k)).length);
    const weg = alleWeg.filter(([, f]) => S.wegFilter === "alle" || f.g.startsWith(S.wegFilter));
    v.append(h("h2", {}, "Aussortiert (" + alleWeg.length + ")"));
    if (!alleWeg.length) {
      v.append(h("p", { class: "still" }, "Es wurde nichts aussortiert."));
    } else {
      const sel = h("select", { id: "weg-filter", "aria-label": "Grund filtern" },
        KAT.filter(([k]) => k === "alle" || zaehleKat(k) > 0).map(([k, t]) => h("option", { value: k, selected: S.wegFilter === k }, t + " (" + zaehleKat(k) + ")")));
      sel.addEventListener("change", () => { S.wegFilter = sel.value; S.zeigeEntfernt = 60; zeige(); });
      v.append(
        h("p", { class: "still" }, "Zu jedem Foto steht, warum es nicht im Buch ist. Klicke ein Foto an für die grosse Ansicht und um es doch ins Buch zu holen."),
        h("div", { class: "zeile" }, h("label", { for: "weg-filter" }, "Grund"), sel),
        h("div", { class: "raster gross", id: "aussortiert", style: "--kg:" + Math.round(S.kg * 0.85) + "px" },
          weg.slice(0, S.zeigeEntfernt).map(([id, f]) => rasterKarte(id, f)))
      );
      if (weg.length > S.zeigeEntfernt) {
        v.append(h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf", onclick: () => { S.zeigeEntfernt += 120; zeige(); } }, "Mehr anzeigen")));
      }
    }
    const nichtLesbar = Object.values(a.fotos).filter((f) => f.x).length;
    if (nichtLesbar) v.append(h("div", { class: "warn" }, nichtLesbar + " Dateien konnten nicht gelesen werden und sind nicht im Buch."));
    if (a.zahlen.eigene_entscheidungen) {
      v.append(h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf", id: "btn-reset", onclick: () => aktion(async () => { await api("/api/entscheidungen/zuruecksetzen", {}); await ladeUndZeige(); }) }, "Alle meine Entscheidungen zurücksetzen (" + a.zahlen.eigene_entscheidungen + ")")));
    }
    v.append(fuss(2, 4, "Weiter zu den Seiten →"));
    return v;
  }

  function karteUrl(o) {
    return "https://www.openstreetmap.org/?mlat=" + o.lat + "&mlon=" + o.lon + "#map=17/" + o.lat + "/" + o.lon;
  }

  function viewOrte(a) {
    const zo = a.zahlen_orte;
    const out = [h("h2", {}, "Orte")];
    if (!zo.mit_gps) {
      out.push(h("p", { class: "still" }, "Keines der Fotos hat GPS-Daten. Darum gibt es hier nichts zu prüfen."));
      return out;
    }
    const max = h("input", { type: "number", id: "max-ort", min: 1, max: 500, step: 1, value: a.einstellungen.max_pro_ort || "", placeholder: "aus", "aria-label": "Höchstens so viele Fotos pro Ort" });
    max.addEventListener("change", () =>
      aktion(async () => {
        await api("/api/einstellungen", { max_pro_ort: max.value || null });
        await ladeUndZeige();
      })
    );
    out.push(
      h("p", { class: "still" }, "Fotos vom gleichen Ort sind hier zusammengefasst und nach Qualität gereiht (Q = Schärfe und Auflösung, 100 ist am besten). So siehst du, welche Fotos eines Ortes die besten sind."),
      hilfe(
        "Wozu Orte, und was bewirkt die Begrenzung?",
        h("p", {}, "Ein Ort ist ein Umkreis von " + a.einstellungen.ort_radius_m + " Metern um die GPS-Position. Wer am Times Square 60 Fotos macht, soll im Buch nicht 60 Fotos davon haben. Mit «Höchstens N Fotos pro Ort» bleiben pro Ort nur die N besten, die übrigen sind vorsortiert entfernt."),
        h("p", {}, "Du kannst jedes Foto trotzdem mit einem Klick zurückholen oder entfernen. Eigene Entscheidungen zählen immer mehr als die Begrenzung. Ortsnamen kennt die App nicht, weil sie offline arbeitet: Gib den Orten selbst einen Namen.")
      ),
      h("div", { class: "zeile" }, h("label", { for: "max-ort" }, "Höchstens"), max, h("span", {}, "Fotos pro Ort (leer = keine Begrenzung)")),
      h("div", { class: "chips" },
        chip(zo.orte, "Orte"),
        chip(zo.mit_gps, "Fotos mit GPS"),
        chip(zo.ohne_ort, "Fotos ohne Ort"),
        chip(zo.ort_entfernt, "wegen der Begrenzung entfernt"))
    );
    const orte = a.orte.slice(0, S.zeigeOrte);
    for (const o of orte) {
      const name = h("input", { type: "text", "data-ortkey": o.key, value: o.name, placeholder: o.standardname, maxlength: 80, "aria-label": "Name von " + o.standardname });
      name.addEventListener("change", () =>
        aktion(async () => {
          await api("/api/ort/name", { key: o.key, name: name.value });
          await ladeUndZeige();
        })
      );
      out.push(
        h("div", { class: "ort", "data-ort": o.index },
          h("div", { class: "ort-kopf" },
            name,
            h("span", { class: "still" }, o.im_buch + " von " + o.anzahl + " Fotos im Buch · " + zeitraum(o.von, o.bis)),
            h("a", { href: karteUrl(o), target: "_blank", rel: "noopener noreferrer", title: "Öffnet openstreetmap.org im Browser. Die App selbst sendet nichts." }, "Karte ↗")),
          h("div", { class: "raster ort-raster" }, o.beste.map((id) => ortFoto(id))),
          o.anzahl > o.beste.length ? h("p", { class: "klein still" }, "Gezeigt sind die " + o.beste.length + " besten von " + o.anzahl + " Fotos.") : null)
      );
    }
    if (a.orte.length > S.zeigeOrte) {
      out.push(h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf", id: "btn-mehr-orte", onclick: () => { S.zeigeOrte += 12; zeige(); } }, "Weitere Orte anzeigen (" + (a.orte.length - S.zeigeOrte) + ")")));
    }
    return out;
  }

  function ortFoto(id) {
    const f = S.a.fotos[id];
    return h(
      "div",
      {
        class: "karte " + (f.b ? "behalten" : "weg"),
        "data-id": id,
        role: "button",
        tabindex: 0,
        "aria-pressed": f.b ? "true" : "false",
        title: f.n + " · Rang " + f.r + " im Ort · " + (f.b ? "im Buch" : f.g || "entfernt"),
        onclick: () => aktion(async () => { await api("/api/foto", { id, aktion: f.b ? "entfernen" : "behalten" }); await ladeUndZeige(); }),
        onkeydown: (ev) => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); ev.currentTarget.click(); } },
      },
      h("img", { src: vorschauUrl(id, 200), alt: f.n, loading: "lazy" }),
      h("span", { class: "marke-b" }, "#" + f.r),
      h("div", { class: "info" }, h("span", {}, "Q " + f.q), h("span", {}, f.b ? "im Buch" : "entfernt"))
    );
  }

  function chip(wert, text) {
    return h("div", { class: "chip" }, h("b", {}, wert), text);
  }

  function serienKarte(id, i, g) {
    const f = S.a.fotos[id];
    const behalten = f.b;
    return h(
      "div",
      {
        class: "karte " + (behalten ? "behalten" : "weg") + (i === S.fokus ? " fokus" : ""),
        "data-id": id,
        role: "button",
        tabindex: 0,
        "aria-pressed": behalten ? "true" : "false",
        "aria-label": f.n + (behalten ? ", im Buch" : ", entfernt"),
        onclick: () => { S.fokus = i; toggleFoto(g, id); },
      },
      h("img", { src: vorschauUrl(id, 400), alt: f.n, loading: "lazy" }),
      h("span", { class: "marke-b" }, behalten ? "Im Buch" : "Entfernt"),
      id === g.vorschlag ? h("span", { class: "marke-v" }, "Vorschlag") : null,
      h("button", { type: "button", class: "lupe-knopf", "aria-label": "Gross ansehen", onclick: (ev) => { ev.stopPropagation(); lupe(id); } }, "🔍"),
      h("div", { class: "info" }, h("span", {}, f.n), h("span", {}, f.z.slice(11) + " · " + f.mp + " MP · Q " + f.q))
    );
  }

  function rasterKarte(id, f) {
    return h(
      "div",
      {
        class: "karte weg",
        "data-id": id,
        role: "button",
        tabindex: 0,
        title: f.n + " · " + f.gd,
        onclick: () => lupe(id),
        onkeydown: (ev) => { if (ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); ev.currentTarget.click(); } },
      },
      h("img", { src: vorschauUrl(id, 400), alt: f.n, loading: "lazy" }),
      h("div", { class: "grund" }, h("b", {}, f.g || "Entfernt"), h("span", {}, f.gd))
    );
  }

  function toggleFoto(g, id) {
    return aktion(async () => {
      const f = S.a.fotos[id];
      const behalten = g.ids.filter((x) => (x === id ? !f.b : S.a.fotos[x].b));
      await api("/api/serie", { nr: g.nr, waehle: behalten.length ? behalten : "keines" });
      await ladeUndZeige();
    });
  }

  function serieAktion(nr, waehle) {
    return aktion(async () => {
      await api("/api/serie", { nr, waehle });
      await ladeUndZeige();
    });
  }

  function alleVorschlaege() {
    return aktion(async () => {
      await api("/api/serien/alle-vorschlaege", {});
      await ladeUndZeige();
    });
  }

  function springeSerie(i) {
    const n = serienListe().length;
    if (!n) return;
    S.serie = (i + n) % n;
    S.fokus = 0;
    zeige();
  }

  // ------------------------------------------------------------------ Schritt 4: Seiten

  function viewSeiten() {
    const a = S.a;
    const p = a.seitenplan;
    const e = a.einstellungen;
    const prof = a.profil;
    const v = h("div", {});
    v.append(
      h("h1", {}, "Seiten"),
      h("p", { class: "still" }, "Wie viele Fotos passen auf eine Seite, und wie viele Seiten ergibt dein Buch? Der Zähler rechnet mit, während du Fotos auswählst."),
      hilfe(
        "So rechnet der Seitenzähler",
        h("p", {}, "Seiten = Fotos geteilt durch Fotos pro Seite, aufgerundet. Druckdienste erlauben nur bestimmte Seitenzahlen (zum Beispiel ab 24, dann in Zweierschritten). Das Buch hat deshalb immer die nächste erlaubte Seitenzahl."),
        h("p", {}, "Beginnt jedes Ereignis auf einer neuen Seite, bleiben dort oft Plätze frei. Das ist übersichtlicher, braucht aber mehr Seiten."),
        h("p", {}, "Vorschläge machen genau dort Platz oder füllen auf, wo sich eine Seite lohnt. Du entscheidest, ob du sie anwendest.")
      )
    );

    const feld = (id, bez, hin, el) => h("div", { class: "feld" }, h("label", { for: id }, bez), el, hin ? h("span", { class: "hinweis" }, hin) : null);
    const speichere = (daten) => aktion(async () => { await api("/api/einstellungen", daten); await ladeUndZeige(); });

    const fps = h("input", { type: "number", id: "fps", min: 1, max: 8, step: 1, value: e.fotos_pro_seite });
    fps.addEventListener("change", () => speichere({ fotos_pro_seite: Number(fps.value) }));
    const getrennt = h("input", { type: "checkbox", id: "getrennt", checked: e.ereignisse_getrennt });
    getrennt.addEventListener("change", () => speichere({ ereignisse_getrennt: getrennt.checked }));
    const profil = h("select", { id: "profil" }, prof.auswahl.map((n) => h("option", { value: n, selected: n === e.profil }, n)));
    profil.addEventListener("change", () => speichere({ profil: profil.value }));

    v.append(
      h("div", { class: "zeile" },
        feld("fps", "Fotos pro Seite", "Durchschnitt, ein bis acht", fps),
        feld("profil", "Druckdienst", null, profil)),
      h("div", { class: "zeile" }, getrennt, h("label", { for: "getrennt" }, "Jedes Ereignis beginnt auf einer neuen Seite"))
    );
    if (!prof.verifiziert) {
      v.append(h("div", { class: "warn" }, "Die Seitenregeln dieses Profils sind noch nicht geprüft (" + prof.seiten_min + " Seiten, Schritt " + prof.seiten_schritt + ", höchstens " + prof.seiten_max + "). Prüfe sie bei deinem Druckdienst und passe sie unten an." + (prof.hinweis ? " " + prof.hinweis : "")));
    }

    const mn = h("input", { type: "number", id: "seiten-min", min: 1, max: 1000, value: e.seiten_min || "", placeholder: prof.seiten_min });
    const st = h("input", { type: "number", id: "seiten-schritt", min: 1, max: 1000, value: e.seiten_schritt || "", placeholder: prof.seiten_schritt });
    const mx = h("input", { type: "number", id: "seiten-max", min: 1, max: 1000, value: e.seiten_max || "", placeholder: prof.seiten_max });
    const regeln = () => speichere({ seiten_min: mn.value || null, seiten_schritt: st.value || null, seiten_max: mx.value || null });
    [mn, st, mx].forEach((x) => x.addEventListener("change", regeln));
    v.append(
      h("details", { class: "hilfe" }, h("summary", {}, "Seitenregeln anpassen"),
        h("div", { class: "zeile" }, feld("seiten-min", "Mindestens", null, mn), feld("seiten-schritt", "Schrittweite", null, st), feld("seiten-max", "Höchstens", null, mx)),
        h("p", { class: "hinweis still" }, "Leer lassen übernimmt den Wert des Profils."))
    );

    // Zähler
    const zaehler = h("div", { class: "zaehler", id: "zaehler" },
      h("div", { class: "gross" }, p.buch_seiten + " Seiten"),
      h("p", {}, zahl(p.fotos) + " Fotos ergeben " + p.seiten_benoetigt + " Seiten" + (p.leere_seiten ? ", " + (p.leere_seiten === 1 ? "1 Seite bleibt" : p.leere_seiten + " Seiten bleiben") + " leer." : ".")));
    if (p.ueber_maximum) zaehler.append(h("div", { class: "warn" }, "Das sind mehr Seiten als erlaubt (" + prof.seiten_max + "). Entferne Fotos oder erhöhe Fotos pro Seite."));
    else if (p.fehlende_fotos === 0) zaehler.append(h("div", { class: "gut" }, "Alle " + p.buch_seiten + " Seiten sind gefüllt."));
    else zaehler.append(h("p", {}, "Um alle " + p.buch_seiten + " Seiten zu füllen, fehlen ", h("b", { id: "fehlend" }, p.fehlende_fotos), " Fotos."));
    if (p.kleinere_stufe) zaehler.append(h("p", {}, "Mit ", h("b", { id: "weglassen" }, p.weglassen_fuer_kleinere_stufe), (p.weglassen_fuer_kleinere_stufe === 1 ? " Foto weniger passt das Buch auf " : " Fotos weniger passt das Buch auf "), h("b", {}, p.kleinere_stufe), " Seiten."));
    v.append(zaehler);

    // Vorschläge
    const vs = a.vorschlaege;
    if (vs.auffuellen.length) {
      v.append(vorschlagBlock("auffuellen", "Auffüllen", (vs.auffuellen.length === 1 ? "Dieses Foto aus einer aussortierten Serie füllt" : "Diese " + vs.auffuellen.length + " Fotos aus aussortierten Serien füllen") + " freie Plätze, ohne eine Seite zu verändern.", vs.auffuellen, "Alle ins Buch nehmen"));
    }
    if (vs.kuerzen.length) {
      v.append(vorschlagBlock("kuerzen", "Kürzen auf " + p.kleinere_stufe + " Seiten", (vs.kuerzen.length === 1 ? "Dieses Foto ist das schwächste" : "Diese " + vs.kuerzen.length + " Fotos sind die schwächsten") + " an den Stellen, wo dadurch eine Seite wegfällt.", vs.kuerzen, "Alle entfernen"));
    }

    // Tabelle
    v.append(
      h("h2", {}, "Seiten je Ereignis"),
      h("table", { id: "tabelle" },
        h("thead", {}, h("tr", {}, h("th", {}, "Ereignis"), h("th", { class: "z" }, "Fotos"), h("th", { class: "z" }, "Seiten"))),
        h("tbody", {}, a.ereignisse.map((ev) => h("tr", {}, h("td", {}, ereignisName(ev)), h("td", { class: "z" }, ev.im_buch), h("td", { class: "z" }, ev.seiten)))))
    );
    v.append(fuss(3, 5, "Weiter zur Buchansicht →"));
    return v;
  }

  function vorschlagBlock(art, titel, text, ids, knopf) {
    return h("div", { class: "vorschlag", "data-art": art },
      h("h3", {}, titel),
      h("p", {}, text),
      h("div", { class: "streifen", style: "margin-bottom:10px" }, ids.slice(0, 12).map((id) => h("img", { src: vorschauUrl(id, 160), alt: S.a.fotos[id].n, title: S.a.fotos[id].n }))),
      h("button", { type: "button", class: "knopf haupt", id: "btn-" + art, onclick: () => aktion(async () => { await api("/api/vorschlag/anwenden", { art }); await ladeUndZeige(); }) }, knopf));
  }

  // ------------------------------------------------------------------ Schritt 5: Export

  function viewExport() {
    const a = S.a;
    const p = a.seitenplan;
    const v = h("div", {});
    v.append(
      h("h1", {}, "Export"),
      h("p", { class: "still" }, "Der Assistent kopiert die Fotos in nummerierter Reihenfolge, je Ereignis ein Ordner, und schreibt eine Seitenliste. Deine Originale bleiben unverändert."),
      hilfe(
        "Wie geht es beim Druckdienst weiter?",
        h("p", {}, "Starte die Software deines Druckdienstes (zum Beispiel Cewe, Pixum oder Ifolor) und lade die Fotos aus dem Exportordner hoch. Die Nummer am Dateianfang gibt die Reihenfolge vor, die Datei «seitenliste.csv» zeigt, welches Foto auf welche Seite gehört."),
        h("p", {}, "Eine fertige Datei, die direkt in die Software der Druckdienste importiert wird, gibt es noch nicht. Ob das geht, ist noch zu prüfen.")
      ),
      h("div", { class: "chips" }, chip(zahl(a.zahlen.im_buch), "Fotos"), chip(p.buch_seiten, "Seiten"), chip(a.zahlen.ereignisse, "Ereignisse"))
    );
    if (p.ueber_maximum) v.append(h("div", { class: "warn" }, "Das Buch hat mehr Seiten als das Profil erlaubt. Du kannst trotzdem exportieren."));
    if (p.fehlende_fotos) v.append(h("div", { class: "warn" }, "Es fehlen noch " + p.fehlende_fotos + " Fotos bis zur vollen Seitenzahl. Du kannst trotzdem exportieren."));
    const ziel = h("input", { type: "text", id: "ziel", class: "breit", placeholder: a.ordner + "\\fotobuch-export", "aria-label": "Zielordner", value: S.zielPfad || "" });
    ziel.addEventListener("input", () => { S.zielPfad = ziel.value; });
    v.append(
      h("div", { class: "feld" }, h("label", { for: "ziel" }, "Zielordner"), ziel, h("span", { class: "hinweis" }, "Leer lassen für den Standard. Ein bestehender, gefüllter Ordner wird nie überschrieben: es entsteht ein Unterordner mit Zeitstempel.")),
      h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf haupt", id: "btn-export", onclick: () => aktion(async () => { S.exportErgebnis = await api("/api/export", { ziel: ziel.value || null }); zeige(); }) }, "Jetzt exportieren"))
    );
    const j = S.status && S.status.job;
    const seitenJob = j && j.art === "seiten" ? j : null;
    v.append(
      h("h2", {}, "Fertige Seiten"),
      h("p", { class: "still" }, "Der Assistent kann jede Seite mit dem gewählten Layout als Bild (JPEG, 300 dpi) und als eine PDF-Datei mit allen Seiten erzeugen. Das ist die Vorschau deines Buchs und eine Grundlage für den Druck."),
      h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf", id: "btn-seiten", disabled: !!(seitenJob && seitenJob.laeuft), onclick: () => aktion(async () => { await api("/api/export/seiten", { ziel: ziel.value || null }); S.seitenExport = true; await seitenPoll(); }) }, "Fertige Seiten erzeugen"))
    );
    if (seitenJob && seitenJob.laeuft) v.append(h("div", { id: "seiten-fortschritt", class: "still" }, "Seite " + seitenJob.fertig + " von " + seitenJob.gesamt + " wird erzeugt …"));
    if (seitenJob && seitenJob.fehler) v.append(h("div", { class: "warn" }, seitenJob.fehler));
    if (seitenJob && !seitenJob.laeuft && seitenJob.ergebnis) {
      v.append(h("div", { class: "gut", id: "seiten-ergebnis" }, "Fertig: " + seitenJob.ergebnis.seiten + " Seiten (" + seitenJob.ergebnis.breite + " × " + seitenJob.ergebnis.hoehe + " Pixel) und die PDF «fotobuch.pdf» in " + seitenJob.ergebnis.ordner));
    }
    const r = S.exportErgebnis;
    if (r) {
      v.append(
        h("div", { class: "gut", id: "export-ergebnis" }, "Fertig: " + r.kopiert + " Fotos kopiert nach " + r.ziel + ". Die Seitenliste liegt als «seitenliste.csv» dabei."),
        ...(r.fehler.length ? [h("div", { class: "warn" }, "Nicht kopiert: " + r.fehler.join("; "))] : [])
      );
    }
    v.append(fuss(5, null), h("div", { class: "zeile" }, h("button", { type: "button", class: "knopf", id: "btn-beenden", onclick: () => aktion(async () => { await api("/api/beenden", {}); S.beendet = true; zeige(); }) }, "App beenden")));
    return v;
  }

  // ------------------------------------------------------------------ Lupe

  function lupe(id) {
    const f = S.a.fotos[id];
    const l = $("#lupe");
    const zu = () => { l.hidden = true; };
    l.replaceChildren(
      h("div", { class: "lupe-inhalt" },
        h("img", { src: vorschauUrl(id, 1400), alt: f.n }),
        h("div", { class: "lupe-info" },
          h("b", {}, f.n),
          h("p", { class: "klein-zeile" }, f.z + " · " + f.mp + " MP · Qualität " + f.q + (f.ort ? " · " + f.ort : "")),
          h("p", { id: "lupe-grund", class: f.b ? "gut" : "warn" }, (f.b ? "Im Buch. " : "Nicht im Buch. ") + f.gd),
          h("div", { class: "zeile" },
            h("button", { type: "button", class: "knopf haupt", id: "lupe-umschalten", onclick: (ev) => { ev.stopPropagation(); zu(); aktion(async () => { await api("/api/foto", { id, aktion: f.b ? "entfernen" : "behalten" }); await ladeUndZeige(); }); } }, f.b ? "Aus dem Buch entfernen" : "Ins Buch holen"),
            f.b ? h("button", { type: "button", class: "knopf", id: "lupe-ganz", title: f.pano ? "Dieses Foto ist ein Panorama und bekommt automatisch eine eigene Seite." : "Das Foto bekommt eine eigene Seite", onclick: (ev) => { ev.stopPropagation(); zu(); aktion(async () => { await api("/api/foto/ganzseite", { id, ganz: !f.gz }); await ladeUndZeige(); }); } }, f.gz ? "Nicht mehr als ganze Seite" : "Als ganze Seite") : null,
            h("button", { type: "button", class: "knopf", id: "lupe-zu", onclick: (ev) => { ev.stopPropagation(); zu(); } }, "Schliessen (Esc)")))
      )
    );
    l.hidden = false;
    l.onclick = (ev) => { if (ev.target === l) zu(); };
  }

  // ------------------------------------------------------------------ Schritt 5: Buch (Seitenansicht)

  function seiteElement(sp, gross) {
    const el = h("div", { class: "seite" + (sp.frei ? " frei" : ""), "data-nr": sp.nr, style: "aspect-ratio:" + S.a.buch.ar });
    for (const p of sp.fotos) {
      const f = S.a.fotos[p.id];
      el.append(
        h("button", { type: "button", class: "sfoto", "data-id": p.id, title: f.n + " · " + f.z, "aria-label": f.n + ", Details", style: "left:" + p.x + "%;top:" + p.y + "%;width:" + p.w + "%;height:" + p.h + "%", onclick: (ev) => { if (S.buchAnsicht === "doppel") { ev.stopPropagation(); lupe(p.id); } } },
          h("img", { src: vorschauUrl(p.id, gross ? 900 : 360), alt: f.n, loading: "lazy" }))
      );
    }
    if (sp.frei) el.append(h("span", { class: "frei-text" }, "Freie Seite"));
    return el;
  }

  // Vorlagenauswahl wie in PowerPoint: kleine Vorschauen aller Layouts, ein Klick wählt.
  function layoutPanel(sp) {
    const panel = h("div", { class: "layoutwahl", "data-key": sp.key }, h("span", { class: "still" }, "Layouts werden berechnet …"));
    api("/api/seite/layouts?key=" + encodeURIComponent(sp.key)).then((d) => {
      panel.replaceChildren(
        h("div", { class: "layoutkopf" },
          h("b", {}, "Layout für Seite " + sp.nr),
          h("button", { type: "button", class: "knopf klein-knopf", id: "layout-zu", onclick: () => { S.layoutOffen = null; zeige(); } }, "Schliessen")),
        h("div", { class: "layoutliste" },
          d.layouts.map((felder, i) =>
            h("button", { type: "button", class: "layoutoption" + (i === d.aktuell ? " aktiv" : ""), "data-i": i, title: "Layout " + (i + 1) + (i === 0 ? " (Vorschlag)" : ""), "aria-pressed": i === d.aktuell ? "true" : "false",
              onclick: () => aktion(async () => { await api("/api/seite/layout", { key: sp.key, wahl: i }); await ladeUndZeige(); }) },
              h("span", { class: "minilayout", style: "aspect-ratio:" + d.ar },
                felder.map((f, k) => h("span", { class: "minifoto", style: "left:" + f[0] + "%;top:" + f[1] + "%;width:" + f[2] + "%;height:" + f[3] + "%" },
                  h("img", { src: vorschauUrl(d.fotos[k], 160), alt: "" })))),
              h("span", { class: "minibez" }, i === 0 ? "Vorschlag" : felder.length === 1 && felder[0][2] === 100 ? "Randlos" : "Layout " + (i + 1))))));
    }).catch((e) => meldung(e.message));
    return panel;
  }

  function viewBuch() {
    const a = S.a;
    const b = a.buch;
    const alle = b.seiten.concat(b.freie_seiten.map((nr) => ({ nr, frei: true, fotos: [], titel: "Freie Seite" })));
    const n = alle.length;
    const maxPos = Math.floor(n / 2); // Seite 1 steht allein, danach Doppelseiten
    S.buchPos = Math.max(0, Math.min(S.buchPos, maxPos));
    const speichere = (daten) => aktion(async () => { await api("/api/einstellungen", daten); await ladeUndZeige(); });
    const v = h("div", {});
    v.append(
      h("h1", {}, "Buch"),
      h("p", { class: "still" }, "So würde dein Buch aussehen. Die Reihenfolge folgt der Aufnahmezeit. Pro Seite wählt der Assistent die Anordnung, bei der die Fotos möglichst gross sind, nichts beschnitten wird und Hoch- und Querformate zusammenpassen. Klicke ein Foto an für Details oder zum Entfernen."),
      hilfe("Wie wird angeordnet?",
        h("ul", {},
          h("li", {}, "Jedes Ereignis beginnt auf einer neuen Seite (einstellbar im Schritt «Seiten»)."),
          h("li", {}, "Fotos einer Seite bleiben in zeitlicher Reihenfolge. Das Layout (nebeneinander, untereinander, verschachtelt) wird je nach Hoch- oder Querformat gewählt."),
          h("li", {}, "Es wird nicht beschnitten, die Fotos behalten ihr Seitenverhältnis."),
          h("li", {}, "Freie Seiten am Ende gibt es, weil Druckdienste nur bestimmte Seitenzahlen erlauben.")))
    );
    const fmt = h("select", { id: "seitenformat", "aria-label": "Seitenformat" },
      [["quer", "Quer (4:3)"], ["quadrat", "Quadratisch"], ["hoch", "Hoch (3:4)"]].map(([w, t]) => h("option", { value: w, selected: a.einstellungen.seitenformat === w }, t)));
    fmt.addEventListener("change", () => speichere({ seitenformat: fmt.value }));
    const fps = h("input", { type: "number", id: "bf-fps", min: 1, max: 8, step: 1, value: a.einstellungen.fotos_pro_seite });
    fps.addEventListener("change", () => speichere({ fotos_pro_seite: Number(fps.value) }));
    v.append(
      h("div", { class: "zeile buchleiste" },
        h("label", { for: "seitenformat" }, "Seitenformat"), fmt,
        h("label", { for: "bf-fps" }, "Fotos pro Seite (Durchschnitt)"), fps,
        h("span", { class: "still" }, a.zahlen.im_buch + " Fotos auf " + b.seiten.length + " Seiten" + (b.freie_seiten.length ? " + " + b.freie_seiten.length + " freie" : "")),
        h("span", { class: "abstand" }),
        h("button", { type: "button", class: "knopf" + (S.buchAnsicht === "doppel" ? " haupt" : ""), id: "btn-ansicht-doppel", onclick: () => { S.buchAnsicht = "doppel"; zeige(); } }, "Blättern"),
        h("button", { type: "button", class: "knopf" + (S.buchAnsicht === "uebersicht" ? " haupt" : ""), id: "btn-ansicht-uebersicht", onclick: () => { S.buchAnsicht = "uebersicht"; zeige(); } }, "Übersicht"))
    );
    if (!n) {
      v.append(h("div", { class: "leer" }, "Noch keine Fotos im Buch."));
    } else if (S.buchAnsicht === "uebersicht") {
      const zoom = h("input", { type: "range", id: "zoom", min: 100, max: 420, step: 10, value: S.zoom, "aria-label": "Grösse der Seiten" });
      zoom.addEventListener("input", () => {
        S.zoom = Number(zoom.value);
        const g = $("#uebersicht");
        if (g) g.style.setProperty("--zoom", S.zoom + "px");
      });
      v.append(
        h("div", { class: "zeile" }, h("label", { for: "zoom" }, "Grösse"), zoom, h("span", { class: "still" }, "Klicke eine Seite, um sie gross zu sehen.")),
        h("div", { class: "uebersicht", id: "uebersicht", style: "--zoom:" + S.zoom + "px" },
          alle.map((sp, i) => h("div", { class: "seitenkarte", "data-nr": sp.nr, role: "button", tabindex: 0, onclick: () => { S.buchPos = Math.ceil(i / 2); S.buchAnsicht = "doppel"; zeige(); window.scrollTo(0, 0); }, onkeydown: (ev) => { if (ev.key === "Enter") ev.currentTarget.click(); } },
            seiteElement(sp, false),
            h("div", { class: "klein" }, "Seite " + sp.nr + " · " + sp.titel))))
      );
    } else {
      const links = S.buchPos === 0 ? null : alle[S.buchPos * 2 - 1];
      const rechts = alle[S.buchPos * 2];
      const sichtbar = [links, rechts].filter(Boolean);
      const bezeichnung = sichtbar.length === 2 ? "Seiten " + links.nr + "–" + rechts.nr : "Seite " + sichtbar[0].nr;
      const goto = h("input", { type: "number", id: "buch-goto", min: 1, max: n, value: sichtbar[0].nr, "aria-label": "Zu Seite springen" });
      goto.addEventListener("change", () => { S.buchPos = Math.ceil((Math.max(1, Math.min(n, Number(goto.value) || 1)) - 1) / 2); zeige(); });
      const layoutKnopf = (sp) => sp.layouts > 1
        ? h("button", { type: "button", class: "knopf klein-knopf layout-knopf", "data-nr": sp.nr, "aria-expanded": S.layoutOffen === sp.key ? "true" : "false", title: "Vorlage für die Anordnung der Fotos auf dieser Seite wählen", onclick: () => { S.layoutOffen = S.layoutOffen === sp.key ? null : sp.key; zeige(); } }, "Layout wählen ▾")
        : null;
      v.append(
        h("div", { class: "zeile" },
          h("button", { type: "button", class: "knopf", id: "btn-buch-zurueck", disabled: S.buchPos === 0, onclick: () => { S.buchPos--; zeige(); } }, "← Zurück"),
          h("b", { id: "buch-pos" }, bezeichnung + " von " + n),
          h("button", { type: "button", class: "knopf", id: "btn-buch-weiter", disabled: S.buchPos >= maxPos, onclick: () => { S.buchPos++; zeige(); } }, "Weiter →"),
          h("label", { for: "buch-goto" }, "Gehe zu Seite"), goto),
        h("div", { class: "doppelseite", id: "doppelseite", style: "--ar:" + b.ar },
          [links, rechts].map((sp) => sp
            ? h("figure", { class: "seitenspalte" }, seiteElement(sp, true),
                h("figcaption", {}, "Seite " + sp.nr + " · " + sp.titel + (sp.ganz ? " · ganze Seite" : sp.fotos.length ? " · " + pl(sp.fotos.length, "Foto", "Fotos") : ""), " ", layoutKnopf(sp)),
                S.layoutOffen && S.layoutOffen === sp.key ? layoutPanel(sp) : null)
            : h("figure", { class: "seitenspalte leer-seite" }))),
        h("div", { class: "tastatur" }, h("kbd", {}, "←"), " ", h("kbd", {}, "→"), " blättern · Seite 1 steht allein, danach folgen Doppelseiten wie im gedruckten Buch · «Layout wählen» zeigt Vorlagen zur Anordnung einer Seite · Klick auf ein Foto: gross ansehen, mit Begründung, entfernen oder zurückholen")
      );
    }
    v.append(fuss(4, 6, "Weiter zum Export →"));
    return v;
  }

  // ------------------------------------------------------------------ Anzeige

  function zeige() {
    const inhalt = $("#inhalt");
    $("#version").textContent = S.status ? "Version " + S.status.version : "";
    if (S.beendet) {
      inhalt.replaceChildren(h("h1", {}, "Beendet"), h("p", {}, "Die App wurde beendet. Du kannst dieses Fenster schliessen. Deine Auswahl ist gespeichert."));
      $("#buch").hidden = true;
      return;
    }
    if (!S.status) return;
    if (S.schritt > 1 && !(S.a && S.a.analysiert)) S.schritt = 1;
    let v;
    switch (S.schritt) {
      case 2: v = viewEreignisse(); break;
      case 3: v = viewAussortieren(); break;
      case 4: v = viewSeiten(); break;
      case 5: v = viewBuch(); break;
      case 6: v = viewExport(); break;
      default: v = viewStart();
    }
    inhalt.replaceChildren(v);
    const rahmen = document.querySelector(".rahmen");
    if (rahmen) rahmen.classList.toggle("breit", S.schritt === 5);
    zeigeNavigation();
    zeigeBuch();
  }

  // ------------------------------------------------------------------ Tastatur

  document.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") {
      const l = $("#lupe");
      if (l && !l.hidden) { l.hidden = true; return; }
    }
    const t = ev.target;
    if (t && (t.tagName === "INPUT" || t.tagName === "SELECT" || t.tagName === "TEXTAREA")) return;
    if (ev.ctrlKey || ev.metaKey || ev.altKey) return;
    if (S.schritt === 5 && S.a && S.a.buch && S.buchAnsicht === "doppel") {
      const maxPos = Math.floor((S.a.buch.seiten.length + S.a.buch.freie_seiten.length) / 2);
      if (ev.key === "ArrowRight" && S.buchPos < maxPos) { S.buchPos++; zeige(); ev.preventDefault(); }
      else if (ev.key === "ArrowLeft" && S.buchPos > 0) { S.buchPos--; zeige(); ev.preventDefault(); }
      return;
    }
    if (S.schritt !== 3 || !S.a || !S.a.serien || !S.a.serien.length) return;
    if (t && t.tagName === "BUTTON" && (ev.key === " " || ev.key === "Enter")) return;
    const g = S.a.serien[S.serie];
    const k = ev.key.toLowerCase();
    if (k === "arrowright") { S.fokus = (S.fokus + 1) % g.ids.length; zeige(); ev.preventDefault(); }
    else if (k === "arrowleft") { S.fokus = (S.fokus - 1 + g.ids.length) % g.ids.length; zeige(); ev.preventDefault(); }
    else if (k === " " || k === "enter") { ev.preventDefault(); toggleFoto(g, g.ids[S.fokus]); }
    else if (k === "n" || k === "pagedown") springeSerie(S.serie + 1);
    else if (k === "p" || k === "pageup") springeSerie(S.serie - 1);
    else if (k === "v") serieAktion(g.nr, "vorschlag");
    else if (k === "g") lupe(g.ids[S.fokus]);
  });

  // ------------------------------------------------------------------ Start

  async function start() {
    try {
      await lade();
      if (S.a && S.a.analysiert) S.schritt = Math.max(1, Math.min(6, S.a.schritt || 2));
      else S.schritt = 1;
      zeige();
      if (S.status.job.laeuft) pollStart();
    } catch (e) {
      meldung(e.message);
    }
  }

  window.__fotobuch = { S, start, zeige, geheZu };
  start();
})();
