(function(){
  var ua=(navigator.userAgent||"")+" "+(navigator.platform||"");
  var os=/Android|iPhone|iPad|iPod/i.test(ua)?"":/Windows/i.test(ua)?"windows":/Mac/i.test(ua)?"mac":/Linux|X11/i.test(ua)?"linux":"";
  var b=document.getElementById("hauptdl"),k=document.getElementById("dl-"+os);
  if(os&&b&&k){var l=k.querySelector("a.knopf");b.href=l.href;b.setAttribute("download","");b.textContent=b.textContent+" – "+b.dataset.fuer.replace("{os}",k.querySelector("h3").textContent);k.classList.add("treffer")}
  document.querySelectorAll(".tabs button").forEach(function(t){t.addEventListener("click",function(){
    document.querySelectorAll(".tabs button").forEach(function(x){x.setAttribute("aria-selected",x===t)});
    document.querySelectorAll(".themapanel").forEach(function(p){p.hidden=p.dataset.t!==t.dataset.t})})});
})();
