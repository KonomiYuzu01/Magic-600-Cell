// Isolated native discovery and binding tools. Commands still use the Shell transaction boundary.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Runtime.InteropServices;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal sealed partial class ExperimentShell {
 static readonly string[] forecastRestrictedCommands=new[]{"assign-a","assign-b","assign-target","block-capture","block-capture-position","block-protect","block-protect-position","block-unprotect-exact","block-unprotect-position","block-unprotect","block-remove","capture-piece"};
 static readonly Color toolPaper=Paper,toolInk=Ink,toolAccent=Accent;
 // Paint-only corners: the existing rectangular hit target and native focus remain intact.
 static void RoundButtonEdges(Button button,Graphics graphics){
  if(button.Width<4||button.Height<4||button.Parent==null)return;
  float radius=Math.Min(6*graphics.DpiX/96f,Math.Min(button.Width,button.Height)/2f),d=radius*2,w=button.ClientSize.Width-1,h=button.ClientSize.Height-1;
  using(var path=new GraphicsPath()){
   path.AddArc(0,0,d,d,180,90);path.AddArc(w-d,0,d,d,270,90);path.AddArc(w-d,h-d,d,d,0,90);path.AddArc(0,h-d,d,d,90,90);path.CloseFigure();
   var saved=graphics.Save();graphics.SmoothingMode=SmoothingMode.AntiAlias;
   using(var outside=new Region(button.ClientRectangle))using(var fill=new SolidBrush(button.Parent.BackColor)){outside.Exclude(path);graphics.FillRegion(fill,outside);}
   if(button.FlatAppearance.BorderSize>0)using(var pen=new Pen(button.FlatAppearance.BorderColor,button.FlatAppearance.BorderSize)){pen.Alignment=PenAlignment.Inset;graphics.DrawPath(pen,path);}
   graphics.Restore(saved);
  }
 }
 static void PaintRoundedButton(object sender,PaintEventArgs e){RoundButtonEdges((Button)sender,e.Graphics);}
 static void UseRoundedButton(Button button){button.Paint-=PaintRoundedButton;button.Paint+=PaintRoundedButton;}
 static void StyleTool(Control root){
  bool errorLabel=root is Label&&(root.ForeColor==Color.FromArgb(172,58,45)||root.ForeColor==Color.DarkRed);
  if(root is Form||root is Panel||root is Label){root.BackColor=toolPaper;root.ForeColor=toolInk;}
  if(root is TextBoxBase||root is ComboBox||root is ListBox){root.BackColor=Color.FromArgb(34,45,56);root.ForeColor=toolInk;}
  var button=root as Button;if(button!=null&&!(button is CellCenterButton)){button.FlatStyle=FlatStyle.Flat;button.BackColor=Color.FromArgb(25,33,41);button.ForeColor=toolInk;button.FlatAppearance.BorderColor=Rule;button.FlatAppearance.MouseOverBackColor=Color.FromArgb(34,45,56);UseRoundedButton(button);}
  if(errorLabel)root.ForeColor=Color.FromArgb(238,141,135);
  foreach(Control child in root.Controls)StyleTool(child);
 }
 Form ToolDialog(string title,int width,int height){return new Form{Text=title,Font=form.Font,BackColor=toolPaper,ForeColor=toolInk,ClientSize=new Size(width,height),MinimumSize=new Size(500,360),Padding=new Padding(12),StartPosition=FormStartPosition.CenterParent,ShowInTaskbar=false,Owner=form};}
 static Button ToolButton(string label){var button=new Button{Text=label,AutoSize=true,FlatStyle=FlatStyle.Flat,BackColor=toolPaper,ForeColor=toolInk,Padding=new Padding(7,3,7,3),Margin=new Padding(3)};UseRoundedButton(button);return button;}
 static Label ToolLabel(string text){return new Label{Text=text,Dock=DockStyle.Fill,AutoEllipsis=true,Padding=new Padding(3,4,3,0)};}
 static FlowLayoutPanel ToolRow(){return new FlowLayoutPanel{Dock=DockStyle.Fill,WrapContents=false,Margin=Padding.Empty};}
 static TableLayoutPanel ToolLayout(Form dialog,params int[] heights){var grid=new TableLayoutPanel{Dock=DockStyle.Fill,ColumnCount=1,RowCount=heights.Length,Margin=Padding.Empty};foreach(int height in heights)grid.RowStyles.Add(new RowStyle(height<0?SizeType.Percent:SizeType.Absolute,height<0?100:height));dialog.Controls.Add(grid);return grid;}
 static ListView ToolList(){var list=new ListView{Dock=DockStyle.Fill,View=View.Details,FullRowSelect=true,MultiSelect=false,HideSelection=false,HeaderStyle=ColumnHeaderStyle.None,BorderStyle=BorderStyle.FixedSingle,BackColor=toolPaper,ForeColor=toolInk};list.Columns.Add("Action",400);list.Columns.Add("Key",130);list.Resize+=delegate{list.Columns[0].Width=Math.Max(190,list.ClientSize.Width-145);};return list;}
 static string ToolSelected(ListView list){return list.SelectedItems.Count==1?Convert.ToString(list.SelectedItems[0].Tag):null;}
 static void ToolSelect(ListView list,string id){foreach(ListViewItem item in list.Items)if(Convert.ToString(item.Tag)==id){item.Selected=true;item.EnsureVisible();return;}}
 static void ToolSearchNavigation(TextBox searchBox,ListView list){searchBox.KeyDown+=delegate(object sender,KeyEventArgs e){if(e.KeyCode!=Keys.Down||list.Items.Count==0)return;list.Focus();if(list.SelectedItems.Count==0)list.Items[0].Selected=true;e.Handled=true;e.SuppressKeyPress=true;};}

 void ShowSelectedMacroDetails(){ShowSelectedMacroDetails(false);}
 void ShowSelectedMacroDetails(bool compareFirst){
  var record=SelectedMacroRecord();var lines=new List<string>{MacroName(record),"Revision "+Text(record["version"]),"",MacroClassificationSummary(record),""};
  if(selectedEffect!=null&&selectedEffectKey==SelectedMacroEffectKey())lines.Add(EffectCaption(selectedEffect));else lines.Add("The complete fixed action has not been checked in this context.");
  lines.Add("");lines.Add("Current applicability · macro body only");
  if(selectedApplicability==null)lines.Add("Recheck this macro after changing the work. No current match is claimed.");
  else{lines.Add("Buffer roles: "+Text(Value(selectedApplicability,"roles"))+" · destination frame: "+Text(Value(selectedApplicability,"frame")));var reasons=Items(Value(selectedApplicability,"reasons"));if(reasons.Length==0)lines.Add("Declared conditions match the macro body.");else lines.AddRange(reasons.Select(reason=>"• "+Text(reason)));}
  lines.Add("");lines.Add("Protection requires a fresh check of Prepare + Macro + Cleanup, including hidden objects. A body match is not permission to execute.");lines.Add("");lines.Add(MacroDetails(record));
  string variant=null;
  using(var dialog=ToolDialog("Macro details",730,540)){
   var grid=ToolLayout(dialog,-1,MacroContextHeight,42);var tabs=new TabControl{Dock=DockStyle.Fill,AccessibleName="Macro details and canonical record"};
   foreach(var item in new[]{new[]{"Effect",String.Join("\r\n",lines)},new[]{"Record",api.Json(record)}}){var page=new TabPage(item[0]);page.Controls.Add(new TextBox{Dock=DockStyle.Fill,ReadOnly=true,Multiline=true,ScrollBars=ScrollBars.Vertical,BorderStyle=BorderStyle.None,Text=item[1].Replace("\r\n","\n").Replace("\n","\r\n"),AccessibleName="Read-only macro "+item[0].ToLowerInvariant()});tabs.TabPages.Add(page);}
   var compare=MacroComparePage(dialog,record);tabs.TabPages.Add(compare);if(compareFirst)tabs.SelectedTab=compare;
   grid.Controls.Add(tabs,0,0);grid.Controls.Add(MacroWorkContext(),0,1);var row=ToolRow();Button inverse=ToolButton("Save inverse…"),reference=ToolButton("Save with R…"),close=ToolButton("Close");close.DialogResult=DialogResult.Cancel;row.Controls.Add(inverse);row.Controls.Add(reference);row.Controls.Add(close);inverse.Click+=delegate{variant="inverse";dialog.Close();};reference.Click+=delegate{variant="reference";dialog.Close();};grid.Controls.Add(row,0,2);dialog.CancelButton=close;ShowOwned(dialog);
  }
  if(variant!=null&&!closing)ShowMacroVariant(variant,record);
 }

 void ShowBankPicker(){
  var records=Items(Value(work,"banks")).Select(Map).ToArray();string active=Text(Value(Workspace,"bank")),accepted=null;
  using(var dialog=ToolDialog("Keyboard bank",670,460)){
   var grid=ToolLayout(dialog,56,36,36,-1,76,42);
   grid.Controls.Add(ToolLabel("Keyboard: "+active+" · "+(Value(Bank,"orbit")==null?"Shared operations":OrbitName(Number(Bank["orbit"])))+"\n"+(FunctionsShortcutNotice().Length==0?"Work: "+OrbitName(Number(Workspace["orbit"]))+" · targets and draft stay in place.":FunctionsShortcutNotice())),0,0);
   var query=new TextBox{Dock=DockStyle.Fill,AccessibleName="Find a bank by exact ID, orbit or purpose"};grid.Controls.Add(query,0,1);
   var orbitPicker=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Dock=DockStyle.Fill,AccessibleName="Orbit bank group"};
   orbitPicker.Items.Add(new Choice("shared","Shared operation sets"));orbitPicker.Items.Add(new Choice("legacy","Legacy sets — existing bindings"));foreach(int id in records.Where(r=>Value(r,"orbit")!=null).Select(r=>Number(r["orbit"])).Distinct())orbitPicker.Items.Add(new Choice(id.ToString(),OrbitName(id)));
   for(int i=0;i<orbitPicker.Items.Count;i++)if(((Choice)orbitPicker.Items[i]).Id==(Object.Equals(Value(Bank,"legacy"),true)?"legacy":Value(Bank,"orbit")==null?"shared":Text(Bank["orbit"])))orbitPicker.SelectedIndex=i;
   grid.Controls.Add(orbitPicker,0,2);var list=ToolList();list.AccessibleName="Five banks for the chosen orbit, or search results";grid.Controls.Add(list,0,3);
   var detail=ToolLabel("");grid.Controls.Add(detail,0,4);var actions=ToolRow();Button activate=ToolButton("Activate selected bank"),cancel=ToolButton("Cancel");cancel.DialogResult=DialogResult.Cancel;actions.Controls.Add(activate);actions.Controls.Add(cancel);grid.Controls.Add(actions,0,5);dialog.CancelButton=cancel;dialog.AcceptButton=activate;
   Action showDetail=delegate{string id=ToolSelected(list);activate.Enabled=id!=null;var record=records.FirstOrDefault(r=>Text(r["id"])==id);if(record==null){detail.Text=list.Items.Count==0?"No matching bank. Clear the search to return to five banks.":"Select one bank to inspect its actual caps and macro slots.";return;}var caps=Items(record["slots"]);detail.Text=id+" · "+record["purpose"]+"\nGrips: "+(caps.Length==0?"capture required":String.Join(", ",caps.Take(6).Select(c=>"C"+c).ToArray())+(caps.Length>6?" … ("+caps.Length+" slots)":""))+"  |  Macros: "+String.Join(", ",Items(record["macros"]).Select(Text).ToArray())+"\n"+record["requirements"];};
   Action fill=delegate{string selected=ToolSelected(list),term=query.Text.Trim();list.BeginUpdate();list.Items.Clear();foreach(var record in records){string id=Text(record["id"]),group=Object.Equals(Value(record,"legacy"),true)?"legacy":Value(record,"orbit")==null?"shared":Text(record["orbit"]),name=Value(record,"orbit")==null?"Shared":OrbitName(Number(record["orbit"]));bool match=term.Length==0?orbitPicker.SelectedItem!=null&&group==((Choice)orbitPicker.SelectedItem).Id:(id+" "+name+" "+record["purpose"]).IndexOf(term,StringComparison.OrdinalIgnoreCase)>=0;if(!match)continue;var item=new ListViewItem(id+" · "+record["purpose"]+(Object.Equals(Value(record,"legacy"),true)?" [legacy]":"")){Tag=id};item.SubItems.Add(id==active?"Active":"");list.Items.Add(item);}ToolSelect(list,term.Length==0?(selected??active):term);if(list.Items.Count==1)list.Items[0].Selected=true;list.EndUpdate();showDetail();};
   query.TextChanged+=delegate{fill();};orbitPicker.SelectedIndexChanged+=delegate{query.Clear();fill();};list.SelectedIndexChanged+=delegate{showDetail();};ToolSearchNavigation(query,list);
   Action accept=delegate{string id=ToolSelected(list);if(id==null)return;if(!connected||busy){detail.Text="Finish or stop the current check before activating a bank. Your search and selection are kept; no input was queued.";return;}accepted=id;dialog.Close();};activate.Click+=delegate{accept();};list.DoubleClick+=delegate{accept();};dialog.Shown+=delegate{query.Focus();};fill();ShowOwned(dialog);
  }
  if(accepted!=null)Send(LocalApi.D("action","bank","id",accepted));
 }

 internal static string[] ContextCommandIds(string kind,string section){
  if(section=="Macro")return new[]{"macro-search","macro-insert","macro-details","macro-compare","macro-inverse","reference-transform","macro-new","phase-macro"};
  if(section=="Operation")return new[]{"phase-input","macro-search","cleanup-inverse","review","operation-focus","worksheet"};
  return kind=="position"?new[]{"roles","focus","block-protect","block-capture","reference","local"}:kind=="identity"?new[]{"focus","next-pin","block-add","capture-piece","copy-selection","local"}:new[]{"focus","orbit","macro-search","bank","filter","worksheet","local"};
 }
 static string ToolCommandGroup(string id){if(id.StartsWith("macro",StringComparison.Ordinal)||id=="reference-transform")return "Macro Base";if(id=="functions-toggle"||id.StartsWith("bank",StringComparison.Ordinal)||id.StartsWith("keyboard",StringComparison.Ordinal)||id=="key-edit")return "Keyboard";if(id.StartsWith("next",StringComparison.Ordinal)||id=="focus"||id=="roles"||id.StartsWith("block",StringComparison.Ordinal))return "Pieces and block";if(new[]{"local","global","puzzle","views","views-close","filter","copy-selection","display","frame-toggle","detail-toggle"}.Contains(id))return "Views and filter";return "Operation and session";}
 string ToolCommandUnavailable(string id){
  if(id.StartsWith("review-reason-",StringComparison.Ordinal)&&!HasReviewReason(Int32.Parse(id.Substring(14))-1))return "Check the complete operation for current evidence first.";
  if(OperationExecuted&&new[]{"macro-insert","macro-replace","cleanup-inverse","review"}.Contains(id))return "These steps were executed. Choose New operation or Reuse steps first.";
  if(OperationExecuted&&new[]{"inspect-prepare","inspect-macro","inspect-cleanup","compare-preview"}.Contains(id))return "These steps were executed. Choose New operation or Reuse steps before inspecting a new result.";
  if((id=="operation-new"||id=="operation-reuse")&&Value(work,"pending")!=null)return "Commit or cancel the pending preview first.";
  if(hub.SelectionIsForecast&&forecastRestrictedCommands.Contains(id))return "Select an actual token or a fixed position first; this token is a forecast.";
  if((id=="next-activate"||id=="next-locate"||id=="next-clear")&&Value(work,"next")==null)return "Pin a Next identity first.";
  if((id=="macro-insert"||id=="macro-details"||id=="macro-compare"||id=="macro-inverse"||id=="reference-transform"||id=="macro-geometry")&&selectedMacro==null)return "Select an existing macro first.";
  if(new[]{"next-pin","block-add","block-capture","block-capture-position","block-protect","block-protect-position","block-unprotect-exact","block-unprotect-position","block-unprotect","block-remove","capture-piece","copy-selection"}.Contains(id)&&Selection==null)return "Inspect a piece or position first.";
  if(id=="preview"&&(Map(Value(work,"review"))==null||Text(Map(work["review"])["status"])!="Ready"))return "A current, policy-compliant complete review is required.";
  if(id=="commit"&&Value(work,"pending")==null)return "Stage the explicitly reviewed operation first.";
  return null;
 }
 void ShowCommandPalette(){
  string chosen=null;bool edit=false;string kind=hub.SelectedKind;int id=hub.SelectedId;
  bool acknowledged=Selection!=null&&(kind=="identity"?Number(Selection["piece"])==id:kind=="position"&&Number(Selection["position"])==id);if(!acknowledged)kind=null;
  Func<string> context=()=>api.Json(new object[]{Value(work,"epoch"),Value(work,"revision"),Value(work,"head"),Value(work,"hash"),hub.SelectedKind,hub.SelectedId,Value(Selection,"piece"),Value(Selection,"position"),Value(Workspace,"bank"),Value(Workspace,"phase"),Value(Workspace,"input"),selectedMacro});string capturedContext=context();
  Func<string,bool> navigation=command=>new[]{"cancel-analysis","macro-search","macro-close","index","bank","keyboard","keyboard-extra","views-close","local","global","puzzle","help"}.Contains(command);
  Func<string,string> blocked=delegate(string command){if(!connected||busy&&!navigation(command))return "Finish or stop the current check first. Your selection is kept; no input was queued.";if(!navigation(command)&&context()!=capturedContext)return "The work context changed while this palette was open. Your selection is kept; close and reopen Commands to inspect the new context before acting.";return ToolCommandUnavailable(command);};
  using(var dialog=ToolDialog("Commands for current work",680,470)){
   var grid=ToolLayout(dialog,36,34,36,-1,58,42);grid.Controls.Add(ToolLabel(kind==null?"Choose a piece, or use the current operation.":"Selected "+(kind=="identity"?"I":"P")+id+" · "+Text(Value(Workspace,"bank"))+" · "+Text(phase)),0,0);
   var query=new TextBox{Dock=DockStyle.Fill,AccessibleName="Search named commands"};grid.Controls.Add(query,0,1);var section=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Dock=DockStyle.Fill,AccessibleName="Command context"};section.Items.AddRange(new object[]{"Selected object","Operation","Macro","All commands"});section.SelectedIndex=kind==null?1:0;grid.Controls.Add(section,0,2);
   var list=ToolList();list.ShowGroups=true;list.AccessibleName="Contextual commands with effective shortcuts";grid.Controls.Add(list,0,3);var detail=ToolLabel("");grid.Controls.Add(detail,0,4);
   var actions=ToolRow();Button run=ToolButton("Use selected action"),change=ToolButton("Change key…"),cancel=ToolButton("Cancel");cancel.DialogResult=DialogResult.Cancel;actions.Controls.Add(run);actions.Controls.Add(change);actions.Controls.Add(cancel);grid.Controls.Add(actions,0,5);dialog.CancelButton=cancel;dialog.AcceptButton=run;
   Action showDetail=delegate{string command=ToolSelected(list);change.Enabled=command!=null;string reason=command==null?null:blocked(command);run.Enabled=command!=null;detail.Text=command==null?(list.Items.Count==0?"No match in this context. Choose All commands to search the complete registry.":"Select an action to see its key and requirements."):hints[command]+"\n"+(reason??("Command: "+command+(KeyHint(command).Length==0?" · no key assigned":KeyHint(command))));};
   Action fill=delegate{string selected=ToolSelected(list),term=query.Text.Trim(),scope=Text(section);var ids=scope=="All commands"?commands.Keys.OrderBy(c=>ToolCommandGroup(c)).ThenBy(c=>hints[c]):ContextCommandIds(kind,scope).Where(commands.ContainsKey).AsEnumerable();list.BeginUpdate();list.Items.Clear();list.Groups.Clear();var groups=new Dictionary<string,ListViewGroup>();foreach(string command in ids){if((hints[command]+" "+command).IndexOf(term,StringComparison.OrdinalIgnoreCase)<0)continue;string group=ToolCommandGroup(command);if(!groups.ContainsKey(group)){groups[group]=new ListViewGroup(group);list.Groups.Add(groups[group]);}var item=new ListViewItem(hints[command],groups[group]){Tag=command};item.SubItems.Add(KeyHint(command).Trim());if(ToolCommandUnavailable(command)!=null)item.ForeColor=Color.FromArgb(112,113,110);list.Items.Add(item);}if(selected!=null)ToolSelect(list,selected);if(list.Items.Count==1)list.Items[0].Selected=true;list.EndUpdate();showDetail();};
   query.TextChanged+=delegate{fill();};section.SelectedIndexChanged+=delegate{fill();};list.SelectedIndexChanged+=delegate{showDetail();};ToolSearchNavigation(query,list);
   Action accept=delegate{string command=ToolSelected(list);if(command==null)return;string reason=blocked(command);if(reason!=null){detail.Text=reason;return;}chosen=command;dialog.Close();};run.Click+=delegate{accept();};list.DoubleClick+=delegate{accept();};change.Click+=delegate{string command=ToolSelected(list);if(command==null)return;if(!connected||busy){detail.Text="Finish or stop the current check before changing a key. Your selection is kept.";return;}if(context()!=capturedContext){detail.Text="The work context changed. Your selection is kept; close and reopen Commands before choosing a bank for key editing.";return;}chosen=command;edit=true;dialog.Close();};dialog.Shown+=delegate{query.Focus();};fill();ShowOwned(dialog);
  }
  if(chosen!=null){if(edit)ShowBindingEditor(chosen);else RunCommand(chosen);}
 }

 internal static string CapturedCommandKey(int scan,bool extended,bool shift,bool ctrl,bool alt,bool meta,bool repeat){
  if(repeat)return null;string code=ExperimentInput.CodeFromScanCode(scan,extended);if(code==null)throw new ArgumentException("Unsupported physical scan code; no binding was changed.");
  if(new[]{"ShiftLeft","ShiftRight","ControlLeft","ControlRight","AltLeft","AltRight","MetaLeft","MetaRight"}.Contains(code))return null;
  if(meta||alt&&(code=="Tab"||code=="F4"))throw new ArgumentException("Choose a key outside Windows window-management shortcuts.");
  if(code=="Enter")throw new ArgumentException("Enter belongs to the focused control and staged execution. Choose another command key.");
  if(!ctrl&&!alt&&(code=="Tab"||code=="Escape"&&!shift))throw new ArgumentException("Escape and Tab remain available for closing and navigating this editor.");
  return (ctrl?"Control+":"")+(alt?"Alt+":"")+(shift?"Shift+":"")+code;
 }
 static string ToolCanonicalKey(string key){return key!=null&&key.StartsWith("Ctrl+",StringComparison.Ordinal)?"Control+"+key.Substring(5):key;}
 internal static Dictionary<string,object> EditedCommandBinding(Dictionary<string,object> original,string bankId,string oldKey,string newKey,string command){
  if(String.IsNullOrEmpty(newKey)||String.IsNullOrEmpty(command))throw new ArgumentException("Capture a key and choose a command first.");
  var serializer=new JavaScriptSerializer{MaxJsonLength=67108864};var result=(Dictionary<string,object>)serializer.DeserializeObject(serializer.Serialize(original??new Dictionary<string,object>()));ValidateBindings(result);
  var group=result;if(bankId!=null){var banks=Map(Value(result,"banks"));if(banks==null){banks=new Dictionary<string,object>();result["banks"]=banks;}group=Map(Value(banks,bankId));if(group==null){group=new Dictionary<string,object>();banks[bankId]=group;}}
  var before=Map(Value(group,"commands"));var normalized=new Dictionary<string,object>();if(before!=null)foreach(var pair in before)normalized[ToolCanonicalKey(pair.Key)]=pair.Value;
  oldKey=ToolCanonicalKey(oldKey);newKey=ToolCanonicalKey(newKey);if(!String.IsNullOrEmpty(oldKey)&&oldKey!=newKey)normalized[oldKey]=null;normalized[newKey]=command;group["commands"]=normalized;ValidateBindings(result);return result;
 }
 sealed class BindingCapture : Control {
  internal event Action<string,string> Captured;
  internal event Action Cancelled;
  internal BindingCapture(){SetStyle(ControlStyles.Selectable|ControlStyles.UserPaint|ControlStyles.AllPaintingInWmPaint|ControlStyles.OptimizedDoubleBuffer,true);TabStop=true;BackColor=toolPaper;ForeColor=toolAccent;AccessibleName="Physical command-key capture; captured keys never execute";Text="Focus here, then press the new key";}
  [DllImport("user32.dll")]static extern short GetKeyState(int key);
  protected override void OnPaint(PaintEventArgs e){base.OnPaint(e);using(var pen=new Pen(Focused?toolAccent:Color.FromArgb(180,184,180),Focused?2:1))e.Graphics.DrawRectangle(pen,1,1,Width-3,Height-3);TextRenderer.DrawText(e.Graphics,Text,Font,new Rectangle(8,5,Width-16,Height-10),ForeColor,TextFormatFlags.VerticalCenter|TextFormatFlags.WordBreak);}
  protected override void OnGotFocus(EventArgs e){base.OnGotFocus(e);Invalidate();}protected override void OnLostFocus(EventArgs e){base.OnLostFocus(e);Invalidate();}
  protected override void OnMouseDown(MouseEventArgs e){base.OnMouseDown(e);Focus();}
  protected override bool IsInputKey(Keys keyData){return true;}
  protected override void WndProc(ref Message message){
   if(message.Msg==0x100||message.Msg==0x104){long bits=message.LParam.ToInt64();var modifiers=ModifierKeys;string code=ExperimentInput.CodeFromScanCode((int)((bits>>16)&255),(bits&(1L<<24))!=0);
    if(code=="Tab"&&(modifiers&~Keys.Shift)==Keys.None){Parent.SelectNextControl(this,(modifiers&Keys.Shift)==0,true,true,true);return;}
    if(code=="Escape"&&modifiers==Keys.None){if(Cancelled!=null)Cancelled();else FindForm().DialogResult=DialogResult.Cancel;return;}
    try{string key=CapturedCommandKey((int)((bits>>16)&255),(bits&(1L<<24))!=0,(modifiers&Keys.Shift)!=0,(modifiers&Keys.Control)!=0,(modifiers&Keys.Alt)!=0,(GetKeyState(0x5B)&0x8000)!=0||(GetKeyState(0x5C)&0x8000)!=0,(bits&(1L<<30))!=0);if(key!=null&&Captured!=null)Captured(key,null);}catch(ArgumentException error){if(Captured!=null)Captured(null,error.Message);}return;
   }
   if(message.Msg==0x101||message.Msg==0x105||message.Msg==0x102||message.Msg==0x106)return;base.WndProc(ref message);
  }
 }
 void ShowBindingEditor(string command){
  if(!connected||busy){Say("Finish the current check and wait for the workspace to connect before changing a key.",true);return;}
  if(!commands.ContainsKey(command))throw new ArgumentException("Unknown command: "+command);string bankId=Text(Value(Workspace,"bank")),newKey=null;
  using(var dialog=ToolDialog("Change command key",650,425)){
   var grid=ToolLayout(dialog,40,34,36,62,60,42,-1,42);grid.Controls.Add(ToolLabel(hints[command]+"\nCommand: "+command),0,0);
   var scope=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Dock=DockStyle.Fill,AccessibleName="Binding scope"};scope.Items.AddRange(new object[]{"This bank: "+bankId,"Shared defaults (bank overrides are retained)"});scope.SelectedIndex=0;grid.Controls.Add(scope,0,1);
   var old=new ComboBox{DropDownStyle=ComboBoxStyle.DropDownList,Dock=DockStyle.Fill,AccessibleName="Existing shortcut to replace, or add another"};grid.Controls.Add(old,0,2);var capture=new BindingCapture{Dock=DockStyle.Fill};grid.Controls.Add(capture,0,3);var status=ToolLabel(command=="functions-toggle"&&FunctionsShortcutNotice().Length>0?"No single-key Functions route in this set. Choose a free key; use Shared scope for a cross-set default. Bank overrides remain.":"Press Tab to reach capture, then press a key. No command runs during capture.");grid.Controls.Add(status,0,4);
   var replace=new CheckBox{Text="Replace the conflicting command in this scope",Dock=DockStyle.Fill,AutoSize=true,Visible=false};grid.Controls.Add(replace,0,5);grid.Controls.Add(ToolLabel(command=="functions-toggle"?"A bank-only edit changes this set. For the same key across sets, use Shared scope and retain an unused physical key; bank overrides still apply.":"Physical Grip/Twist keys remain in their separate reviewed editor. Changes here preserve other banks, caps, recipes and work context."),0,6);
   var actions=ToolRow();Button save=ToolButton("Save binding"),cancel=ToolButton("Cancel");save.Enabled=false;cancel.DialogResult=DialogResult.Cancel;actions.Controls.Add(save);actions.Controls.Add(cancel);grid.Controls.Add(actions,0,7);dialog.CancelButton=cancel;
   Func<Dictionary<string,string>> effective=delegate{if(scope.SelectedIndex==0)return InputState().CommandKeys;var map=new Dictionary<string,string>(defaultKeys);MergeKeys(map,Map(Value(Map(Value(Workspace,"keybinds")),"commands")));return map;};
   Action inspect=delegate{replace.Visible=false;save.Enabled=false;if(newKey==null)return;string conflict;var map=effective();bool commandConflict=map.TryGetValue(newKey,out conflict)&&conflict!=command;string bare=newKey.Substring(newKey.LastIndexOf('+')+1);var inputMap=InputState();bool turnConflict=(newKey==bare||newKey=="Shift+"+bare)&&(inputMap.GripKeys.ContainsKey(bare)||inputMap.TwistKeys.ContainsKey(bare));string oldKey=old.SelectedItem is Choice?((Choice)old.SelectedItem).Id:null;status.Text="Old: "+(String.IsNullOrEmpty(oldKey)?"additional shortcut":oldKey)+"  →  New: "+newKey+(turnConflict?"\nThis key operates Grip/Twist in this set. Choose an unused key or edit a command-only set.":commandConflict?"\nCurrently: "+(hints.ContainsKey(conflict)?hints[conflict]:conflict):"\nAvailable for "+hints[command]+".");replace.Visible=commandConflict&&!turnConflict;save.Enabled=!turnConflict&&(!commandConflict||replace.Checked);};
   Action fillOld=delegate{old.Items.Clear();foreach(string key in effective().Where(p=>p.Value==command).Select(p=>p.Key).OrderBy(k=>k.Length))old.Items.Add(new Choice(key,"Replace "+key));old.Items.Add(new Choice("","Add another shortcut"));old.SelectedIndex=0;replace.Checked=false;inspect();};
   scope.SelectedIndexChanged+=delegate{fillOld();};old.SelectedIndexChanged+=delegate{inspect();};replace.CheckedChanged+=delegate{inspect();};capture.Captured+=delegate(string key,string error){newKey=key;replace.Checked=false;capture.Text=key??"Press another key";capture.Invalidate();if(error!=null){status.Text=error;save.Enabled=false;replace.Visible=false;}else inspect();};
   save.Click+=async delegate{save.Enabled=false;scope.Enabled=false;capture.Enabled=false;old.Enabled=false;replace.Enabled=false;try{if(scope.SelectedIndex==0&&bankId!=Text(Value(Workspace,"bank"))){status.Text="The active bank changed. Your capture is preserved; reopen this editor for the intended bank before saving.";return;}string oldKey=((Choice)old.SelectedItem).Id;var edited=EditedCommandBinding(Map(Value(Workspace,"keybinds")),scope.SelectedIndex==0?bankId:null,oldKey,newKey,command);var checkedValue=CheckedBindings(api.Json(edited));bool accepted=await Send(LocalApi.D("action","settings","keybinds",checkedValue));if(accepted)dialog.Close();else status.Text=feedback.Text;}catch(Exception error){status.Text=error.Message;}finally{if(!dialog.IsDisposed){scope.Enabled=true;capture.Enabled=true;old.Enabled=true;replace.Enabled=true;save.Enabled=true;}}};
   fillOld();dialog.Shown+=delegate{capture.Focus();};ShowOwned(dialog);
  }
 }
}
