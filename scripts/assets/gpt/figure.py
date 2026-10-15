from lib import *

def blob(I, spans, tone, outline='O'):
    cells=set()
    for y,x0,x1 in spans:
        for x in range(x0,x1+1): cells.add((x,y))
    for (x,y) in cells: I.put(x,y,tone(x,y))
    if outline:
        for (x,y) in list(cells):
            for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                if (x+dx,y+dy) not in cells and I.get(x+dx,y+dy)=='.': I.put(x+dx,y+dy,outline)
    return cells

def mirror(spans, w=W):
    return [(y,w-1-x1,w-1-x0) for y,x0,x1 in spans]

def line(I,x0,y0,x1,y1,c,only=None):
    n=max(abs(x1-x0),abs(y1-y0)) or 1
    for i in range(n+1):
        x=round(x0+(x1-x0)*i/n); y=round(y0+(y1-y0)*i/n)
        if only is None or I.get(x,y) in only: I.put(x,y,c)

def curve(I,p0,p1,p2,c,only='ba',n=40):
    for i in range(n+1):
        t=i/n
        x=round((1-t)**2*p0[0]+2*(1-t)*t*p1[0]+t*t*p2[0]); y=round((1-t)**2*p0[1]+2*(1-t)*t*p1[1]+t*t*p2[1])
        if I.get(x,y) in only: I.put(x,y,c)

def draw_hands(I,pose):
    def sleeves(y0,rows=3):
        for k in range(rows):
            I.P(y0+k,16-(k>0),"XXXXX"[: 4+(k>0)]); I.P(y0+k,29,"XXXXX"[: 4+(k>0)])
    if pose=='clasp':
        sleeves(39)
        for y,x0,t in [(39,21,"PPPPPPP"),(40,20,"PsssPsssP"),(41,20,"PstsPstsP"),(42,20,"PssstsssP"),(43,21,"PPPPPPP")]: I.P(y,x0,t)
        I.P(44,23,"g"); I.P(45,23,"n"); I.P(46,23,"g")
    elif pose=='up':      # left hand at the chest, right hand raised with thumb up
        sleeves(40)
        for y,x0,t in [(40,19,"PPPPP"),(41,18,"PsssstP"[:7]),(42,19,"PssssP"),(43,20,"PPPP")]: I.P(y,x0,t)
        for y,x0,t in [(33,35,"PP"),(34,34,"PssP"),(35,34,"PssP"),(36,33,"PPssssP"),(37,33,"PsssssP"),(38,34,"PsssstP"),(39,34,"PPPPPP")]: I.P(y,x0,t)
        I.P(36,36,"ss")
    elif pose=='type':    # both hands low and apart, resting on a keyboard prop
        I.P(41,13,"XXXXXX"); I.P(41,30,"XXXXXX"); I.P(42,12,"XXXXXXX"); I.P(42,30,"XXXXXXX")
        for y,x0,t in [(43,13,"PPPPP"),(44,12,"PsssstP"[:7]),(45,12,"PsssssP"),(46,13,"PPPPP")]: I.P(y,x0,t)
        for y,x0,t in [(43,31,"PPPPP"),(44,30,"PtssssP"),(45,30,"PsssssP"),(46,31,"PPPPP")]: I.P(y,x0,t)
    elif pose=='hold':    # both arms forward, hands under a carried box
        for y,x0,t in [(41,13,"XXXXXXX"),(42,13,"XXXXXXX")]: I.P(y,x0,t); I.P(y,36,t)
        for y,x0,t in [(43,13,"PsssP"),(44,13,"PsssP"),(45,14,"PPP")]: I.P(y,x0,t); I.P(y,35,t)

# ---------------- eyes -----------------
EYES={
 'open':   (["OOOOOO","OwiiiO","OiiIIO","OIIFFO",".sFFs."],["OOOOOO","OiiiwO","OIIIiO","OFFIIO",".sFFs."]),
 'curious':(["OOOOOO","OwwiiO","OiiIIO","OIFFFO",".sFFs."],["OOOOOO","OiiwwO","OIIIiO","OFFFIO",".sFFs."]),
 'focused':(["OOOOOO","OiiiiO","OIIIIO","sFFFFs","......"],["OOOOOO","OiiiiO","OIIIIO","sFFFFs","......"]),
 'shy':    (["......","OOOOOO","OiIIiO","OIwFIO",".OOOO."],["......","OOOOOO","OiIIiO","OIFwIO",".OOOO."]),
 'happy':  (["......","..OO..",".O..O.","O....O","......"],["......","..OO..",".O..O.","O....O","......"]),
 'startled':(["OOOOOO","OwwwwO","OwiiwO","OwiiwO",".OwwO."],["OOOOOO","OwwwwO","OwiiwO","OwiiwO",".OwwO."]),
 'smug':   (["OOOOOO","OOOOOO",".OIIO.","......","......"],["......","..OO..",".O..O.","O....O","......"]),
 'closed': (["......","......","OOOOOO",".OOOO.","......"],["......","......","OOOOOO",".OOOO.","......"]),
 'blink':  (["......","OOOOOO","OIIIIO",".OOOO.","......"],["......","OOOOOO","OIIIIO",".OOOO.","......"]),
}
MOUTH={
 'open':["mm"],'curious':["mm"],'focused':["m"],'shy':["m"],'happy':[".mm.","mMMm"],
 'startled':["mm","MM"],'smug':["mmm"],'closed':["mm"],'blink':["mm"],
}

def build(expr='open', hands='clasp'):
    I=Img()
    # ---- wings (behind the hair, peeking out at the shoulders) ----
    wing=[(27,2,3),(28,1,4),(29,0,5),(30,0,6),(31,0,7),(32,0,8),(33,1,9),(34,1,9),(35,2,9),(36,3,9),(37,5,9),(38,6,9)]
    wt=lambda x,y: 'V' if y<31 else ('U' if y<35 else 'T')
    blob(I,wing,wt); blob(I,mirror(wing),wt)
    for (x,y) in [(1,29),(2,30),(3,31),(4,32),(5,33),(6,34),(7,35)]: I.put(x,y,'L'); I.put(48-x,y,'L')
    for (x,y) in [(3,36),(5,37),(2,35)]: I.put(x,y,'.'); I.put(48-x,y,'.')
    for (x,y) in [(2,31),(3,32),(4,33),(5,34)]: I.put(x,y,'L'); I.put(48-x,y,'L')
    # ---- back hair locks ----
    left=[(y,8,15) for y in range(30,43)]+[(43,9,15),(44,10,15),(45,11,14),(46,12,14)]
    right=[(y,33,40) for y in range(30,43)]+[(43,33,39),(44,33,38),(45,34,37),(46,35,37)]
    def lt(x,y):
        k=x-8                              # 0 = outer edge of the left lock
        if k<=1: return 'd'
        if k in (3,4): return 'c'
        if k==6 and y<40: return 'a'
        return 'b' if k not in (5,) else 'c'
    blob(I,left,lt,'O'); blob(I,right,lambda x,y: lt(48-x+0,y) if False else lt(48-x,y),'O')
    for y in range(36,46):
        if I.get(11,y)=='c' and y%2==0: I.put(11,y,'d')
    # ---- tail (in front of the lock, behind the dress) ----
    def ttone(x,y): return 'S' if (x-38)*0.7+(y-40)*0.5<-2 else ('R' if (x-38)*0.7+(y-40)*0.5<3 else 'Q')
    I.stroke([(35,46),(40,45),(44,42),(46,38),(45,34)],[3.0,3.2,2.8,2.4,1.4],'R','O',ttone)
    for (x,y) in [(41,44),(43,41),(45,38),(40,46),(44,36),(46,34)]:
        if I.get(x,y) in 'SRQ': I.put(x,y,'N')
    for (x,y,c) in [(45,31,'O'),(44,32,'O'),(46,32,'O'),(45,32,'S'),(43,33,'O'),(47,33,'O'),(44,33,'S'),(45,33,'R'),(46,33,'R')]: I.put(x,y,c)
    # ---- dress / body ----
    dress=[(35,19,29),(36,17,31),(37,15,33),(38,14,34),(39,13,35)]+[(y,12,36) for y in range(40,44)]+[(44,11,37),(45,11,37),(46,10,38),(47,10,38)]
    def dt(x,y):
        if x<=15 or x>=33: return 'X'
        if y>=46: return 'Y'
        if x in (19,29) and y>=41: return 'Y'
        return 'W'
    blob(I,dress,dt,'P')
    # neck + collar + bow
    for (y,x0,x1) in [(33,22,26),(34,22,26),(35,22,26)]: I.span(y,x0,x1,'s')
    I.span(35,22,26,'t'); I.P(36,21,"XXWWWXX")
    I.P(36,17,"PPPP"); I.P(36,27,"PPPP")
    I.P(36,18,"ZZY"); I.P(36,28,"YZZ"); I.P(37,19,"ZYYZ"); I.P(37,26,"ZYYZ"); I.P(38,20,"ZZ"); I.P(38,27,"ZZ")
    I.P(37,23,"PgP"); I.P(38,22,"PgwgP")
    draw_hands(I,hands)
    # ---- head hair mass ----
    xl={5:17,6:14,7:12,8:10,9:9,10:8,11:7,12:6,13:5}
    for y in range(14,31): xl[y]=4 if y not in (14,15) else 5
    xr={k:48-v for k,v in xl.items()}
    for y in range(5,31):
        a,b=xl[y],xr[y]
        I.span(y,a,b,'b'); I.put(a,y,'O'); I.put(b,y,'O')
    I.span(5,xl[5],xr[5],'O')
    # tonal blocks: highlight crescents, curved flow ribbons, shaded rims
    for y in range(6,16):
        for x in range(12,36):
            if I.get(x,y)=='b' and ((x-19)**2/70+(y-9)**2/9<1 or (x-32)**2/12+(y-10)**2/5<1): I.put(x,y,'a')
    for p2,ctrl in [((7,25),(9,12)),((10,28),(12,14)),((13,27),(15,15)),((36,26),(33,13)),((39,24),(37,12)),((41,22),(40,10))]:
        curve(I,(22,6),ctrl,p2,'c',only='ba')
    for p2,ctrl in [((8,27),(10,13)),((38,26),(36,12))]:
        curve(I,(23,7),ctrl,p2,'d',only='bac')
    for y in range(8,30):
        for x in list(range(5,9))+list(range(40,44)):
            if I.get(x,y)=='b': I.put(x,y,'c' if (x+y)%3 else 'd')
    for y in range(24,31):
        for x in list(range(5,8))+list(range(41,44)):
            if I.get(x,y) in 'bc': I.put(x,y,'d')
    # ---- face ----
    face=[(22,16,32)]+[(y,15,33) for y in range(23,30)]+[(30,16,32),(31,17,31),(32,19,29),(33,21,27)]
    blob(I,face,lambda x,y:'t' if (y==22 or y>=32) else 's','P')
    I.span(22,16,32,'t')
    # ears (pointed, at eye level)
    I.stroke([(8,27),(4,25),(1,22)],[2.0,1.6,0.5],'s','O',lambda x,y:'s' if y<26 else 't')
    I.stroke([(40,27),(44,25),(47,22)],[2.0,1.6,0.5],'s','O',lambda x,y:'s' if y<26 else 't')
    # ---- eyes / mouth / blush ----
    L,R=EYES[expr]
    for dy,row in enumerate(L):
        for dx,ch in enumerate(row):
            if ch!='.': I.put(16+dx,24+dy,ch)
    for dy,row in enumerate(R):
        for dx,ch in enumerate(row):
            if ch!='.': I.put(27+dx,24+dy,ch)
    for dy,row in enumerate(MOUTH[expr]):
        for dx,ch in enumerate(row):
            if ch!='.': I.put(23+dx,31+dy,ch)
    if expr in('shy','happy'):
        for (x,y) in [(15,30),(16,30),(15,29),(33,30),(32,30),(33,29)]: I.put(x,y,'k')
    else:
        for (x,y) in [(16,29),(32,29)]: I.put(x,y,'k')
    if expr=='startled':
        for (x,y) in [(35,22),(35,23),(36,24)]: I.put(x,y,'F')
    # ---- bangs: clumps with pointed tips ----
    def lock(xa,xb,y0,tip,y1,deep=False):
        for y in range(y0,y1+1):
            f=(y-y0)/max(1,(y1-y0))
            l=round(xa+(tip-xa)*f); r=round(xb+(tip-xb)*f)
            for x in range(l,r+1): I.put(x,y,'b')
            I.put(l,y,'c'); I.put(r,y,'c')
            if r-l>=2 and f<0.55: I.put(l+1,y,'a')
        I.put(tip,y1,'d'); I.put(tip,y1-1,'c')
    for y in range(13,22): I.span(y,7,41,'b')
    lock(8,17,12,13,28)
    lock(13,22,12,17,26)
    lock(18,24,12,21,25)
    lock(24,31,12,25,23)
    lock(29,36,12,31,26)
    lock(34,42,12,36,28)
    lock(38,43,12,41,25)
    for (x,y) in [(18,24),(17,25),(19,22),(26,23),(25,24),(32,25)]: pass
    # ---- ornaments ----
    flower=[(15,38),(16,37),(16,38),(16,39),(17,36),(17,37),(17,38),(17,39),(17,40),(18,37),(18,38),(18,39),(19,38)]
    for (x,y) in flower: I.put(x,y,'a')
    for (x,y) in flower:
        for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
            if I.get(x+dx,y+dy) in 'bcd': I.put(x+dx,y+dy,'P')
    I.put(38,17,'g')
    for i,y in enumerate(range(20,25)): I.put(38,y,'g' if i%2==0 else 'n')
    # ---- horns: ivory crescents sweeping up and out, gold tip, ring ridges ----
    def htone(sign,cx):
        return lambda x,y: 'H' if sign*(x-cx)-y*0.5*sign*0<0 else 'J'
    lh=I.stroke([(15,11),(11,8),(8,4),(9,1)],[3.0,2.7,2.0,0.8],'H','O',lambda x,y: 'H' if (x*0.9-y*0.55)<4.2 else ('J' if (x*0.9-y*0.55)<6.4 else 'K'))
    rh=I.stroke([(33,11),(37,8),(40,4),(39,1)],[3.0,2.7,2.0,0.8],'H','O',lambda x,y: 'H' if ((48-x)*0.9-y*0.55)<4.2 else ('J' if ((48-x)*0.9-y*0.55)<6.4 else 'K'))
    for (x,y) in [(9,2),(39,2)]: I.put(x,y,'G')
    for ring in [(5,9),(7,7),(9,5),(10,3)]: pass
    for (x0,y0,x1,y1) in [(6,6,11,8),(8,3,12,5),(10,9,13,11)]:
        for (cells,mx) in ((lh,False),(rh,True)):
            pass
    for (x,y) in [(7,5),(8,5),(8,4),(9,7),(10,7),(10,8),(11,9),(12,10)]:
        if I.get(x,y) in 'HJ': I.put(x,y,'B')
    for (x,y) in [(41,5),(40,5),(40,4),(39,7),(38,7),(38,8),(37,9),(36,10)]:
        if I.get(x,y) in 'HJ': I.put(x,y,'B')
    # ---- ahoge ----
    I.stroke([(25,6),(25,3),(27,1),(29,2),(29,4)],[0.5,0.5,0.5,0.5,0.5],'c','P')
    return I
