import json
from pathlib import Path
root = Path(__file__).resolve().parent
tokens = json.loads((root / "tokens.json").read_text())
css = (root.parent / "preview/style.css").read_text()
colors = tokens["color"]
def luminance(h):
    rgb = [int(h[i:i+2], 16)/255 for i in (1,3,5)]
    linear = [v/12.92 if v <= .04045 else ((v+.055)/1.055)**2.4 for v in rgb]
    return sum(v*w for v,w in zip(linear, (.2126,.7152,.0722)))
results = []
for fg,bg,minimum in [(c,s,4.5) for c in ("text","muted","verified","refused","failed","unknown","accent") for s in ("canvas","surface","raised")] + [("onAccent","accent",4.5),("border","surface",3),("focus","surface",3)]:
    a,b=sorted((luminance(colors[fg]),luminance(colors[bg])))
    ratio=(b+.05)/(a+.05)
    assert ratio>=minimum,(fg,bg,ratio)
    results.append({"foreground":fg,"background":bg,"ratio":round(ratio,2),"minimum":minimum})
for name,value in colors.items():
    assert f"--{name}:{value}" in css,(name,"CSS drift")
print(json.dumps({"checks":"24 contrast pairs and CSS semantic token consistency","results":results},indent=2))
