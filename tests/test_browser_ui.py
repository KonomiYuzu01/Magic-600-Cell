from pathlib import Path
import json,time,os,shutil
from playwright.sync_api import sync_playwright
from browser_harness import boot,ROOT,INFO
report={'transport':'Real local backend via Python-bound fetch; managed Chromium blocks normal URL navigation','renderer':'SwiftShader under Xvfb','checks':[],'errors':[]}
with sync_playwright() as p:
 b=p.chromium.launch(executable_path=os.environ.get('C600_BROWSER') or shutil.which('chromium') or p.chromium.executable_path,headless=False,args=['--no-sandbox','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader','--disable-dev-shm-usage'])
 page=b.new_page(viewport={'width':1440,'height':940});page.on('pageerror',lambda e:report['errors'].append(str(e)));page.on('dialog',lambda d:d.accept())
 boot(page);page.wait_for_timeout(800)
 def evaluate(s,arg=None):return page.evaluate(s,arg)
 def wait_idle():page.wait_for_function('!document.body.classList.contains("analyzing")',timeout=30000)
 def ok(s):report['checks'].append(s);print('PASS',s,flush=True)
 initial=evaluate('window.c600.state.state_hash')
 assert page.locator('#gripButtons button').count()==7
 page.locator('#gripButtons button').nth(1).click();wait_idle();assert evaluate('!!window.c600.state.pending');page.locator('#quickCommit').click();wait_idle();assert evaluate('window.c600.state.state_hash')!=initial
 page.locator('#undo').click();wait_idle();assert evaluate('window.c600.state.state_hash')==initial
 ok('Focus-cell grip preview, animated commit and exact undo')
 page.locator('#gripButtons button').nth(0).click();wait_idle()
 page.locator('#dockTabs [data-panel="macros"]').click();page.locator('#macroName').fill('UI half turn');page.locator('#saveMacro').click();wait_idle();assert page.locator('#macroLibrary').inner_text().startswith('UI half turn')
 page.locator('#macroA').fill('H0');page.locator('#macroB').fill('T1');page.locator('#commutator').click();wait_idle();assert evaluate('window.c600.state.pending.primitive_count')=='4'
 page.locator('#quickCancel').click();wait_idle();ok('Certified macro library and commutator workbench')
 page.locator('#dockTabs [data-panel="filters"]').click();page.locator('summary',has_text='Named piece sets').click();page.locator('#setName').fill('home0');page.locator('#setExpr').fill('O33 & current(C000)');page.locator('#saveSet').click();wait_idle();assert 'home0' in evaluate('Object.keys(window.c600.state.prefs.named_sets)')
 page.locator('#rules input').fill('set(home0)');page.locator('#applyFilter').click();wait_idle();ok('Named identity set and exact Boolean filter')
 page.locator('#dockTabs [data-panel="view"]').click();page.locator('#bookmarkName').fill('Workbench test');page.locator('#saveBookmark').click();wait_idle();assert 'Workbench test' in evaluate('Object.keys(window.c600.state.prefs.bookmarks)')
 page.locator('#keys').click();page.locator('#keyGrip').click();page.locator('#saveKeys').click();wait_idle();assert evaluate('window.c600.state.prefs.keybinds.Digit1')=='grip1'
 page.locator('#scene').focus();page.keyboard.press('Digit1');wait_idle();assert evaluate('!!window.c600.state.pending');page.keyboard.press('Escape');wait_idle();ok('Camera bookmark and saved grip-focused keyboard profile')
 page.locator('#dockTabs [data-panel="history"]').click();page.locator('#historyRefresh').click();wait_idle();assert page.locator('#historyList>div').count()>=2
 evaluate('async()=>{await window.c600.api("checkpoint",{name:"UI checkpoint"});await window.c600.refresh();}')
 page.locator('#checkpoints').select_option('UI checkpoint');page.locator('#restore').click();wait_idle();assert evaluate('window.c600.state.state_hash')==initial
 ok('History list and checkpoint restore')
 page.locator('#timerToggle').click();wait_idle();page.wait_for_timeout(200);page.locator('#timerToggle').click();wait_idle();assert evaluate('async()=>!(await window.c600.api("stats")).timer.running')
 page.locator('#scene').focus();page.keyboard.press('F8');assert evaluate('document.body.classList.contains("hideDock")');page.keyboard.press('F8');assert not evaluate('document.body.classList.contains("hideDock")')
 ok('Explicit session timer and viewport-only toggle')
 # Build a representative scrambled work-cell screenshot through a legal preview.
 evaluate('async()=>{await window.c600.api("prefs",{rules:[{expr:"current(C000)",style:"solid"}],selected:null});await window.c600.previewRecipe([{kind:"word",moves:[2,4,6,8,10,12,14,16,18,20]}],"UI screenshot scramble");await window.c600.actions.commit();}')
 page.locator('#dockTabs [data-panel="solve"]').click();page.wait_for_timeout(1000);page.screenshot(path=str(ROOT/'docs'/'workbench_02.png'))
 (ROOT/'tests/v02').mkdir(parents=True,exist_ok=True)
 report['state_after']=evaluate('window.c600.state.state_hash');report['passed']=not report['errors'];(ROOT/'tests/v02/ui_flows.json').write_text(json.dumps(report,indent=2));print(json.dumps(report,indent=2),flush=True);b.close()
