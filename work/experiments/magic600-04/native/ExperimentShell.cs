// Native P1 experiments only. No production workbench or automatic solver routes.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Windows.Forms;

internal sealed partial class ExperimentShell : IDisposable, IMessageFilter {
 readonly Form form; readonly LocalApi api; readonly ExperimentBridge bridge; readonly string mode;
 readonly ExperimentHub hub=new ExperimentHub();
 readonly NativeCellView local=new NativeCellView(false),global=new NativeCellView(true);
 readonly Dictionary<string,Action> commands=new Dictionary<string,Action>();
 readonly Dictionary<string,string> hints=new Dictionary<string,string>();
 readonly ComboBox orbit=new ComboBox(),destination=new ComboBox(),phase=new ComboBox(),gripMode=new ComboBox(),workGoal=new ComboBox();
 readonly TextBox search=new TextBox(); readonly ListBox macros=new ListBox();
 readonly Label context=new Label(),protection=new Label(),feedback=new Label(),inputFeedback=new Label(),bankLabel=new Label(),effectText=new Label(),pieceText=new Label();
 readonly Label keyboardContext=new Label();
 readonly FlowLayoutPanel keyboard=new FlowLayoutPanel(); readonly TabControl views=new TabControl();
 readonly Dictionary<string,Button> phaseButtons=new Dictionary<string,Button>();
 readonly CheckBox strict=new CheckBox();
 readonly ToolTip tips=new ToolTip(); readonly List<Form> windows=new List<Form>();
 readonly ExperimentInput input;
 Dictionary<string,object> work,structure,selectedEffect,selectedApplicability,lastCommandResult,lastLocalCell;
 string selectedEffectScope="body",selectedEffectKey;
 NativeSnapshot snapshot;
 bool busy,connected,updating,closing,modal,stopPending;
 string stopFailure;
 readonly bool sendTimingEnabled=Environment.GetEnvironmentVariable("MAGIC600_SEND_TIMING")=="1";
 int sendTimingSequence;long sendTimingKeyHintCalls,sendTimingKeyHintTicks;
 TaskCompletionSource<bool> stopCompletion;
 bool OperationInputBlocked {get{return busy||stopPending||stopFailure!=null;}}
 internal bool StopPending {get{return stopPending;}}
 internal string StopFailure {get{return stopFailure;}}
 Task<bool> WaitForStopCompletion(){return stopCompletion==null?Task.FromResult(true):stopCompletion.Task;}
 string selectedMacro;
 int macroWorkOrbit=-1;
 string macroWorkBinding;
 string linkedPieceContext;
 SplitContainer viewSplit;
 TableLayoutPanel layoutRoot, catalogue;
 FlowLayoutPanel operationStrip;
 Form keyboardWindow;
 readonly List<FlowLayoutPanel> strips=new List<FlowLayoutPanel>();
 readonly Dictionary<Button,string> commandButtons=new Dictionary<Button,string>();
 readonly string[] gripCodes=new[]{"Digit1","Digit2","Digit3","Digit4","Digit5","Digit6","Digit7","Digit8","Digit9","Digit0","KeyQ","KeyW","KeyE","KeyR","KeyT","KeyY","KeyU","KeyI","KeyO","KeyP"};
 readonly Dictionary<string,string> defaultTwists=new Dictionary<string,string>{{"KeyA","H1"},{"KeyS","H2"},{"KeyD","H3"},{"KeyF","T1"},{"KeyG","T2"},{"KeyH","T3"},{"KeyJ","T4"}};
 readonly Dictionary<string,string> defaultKeys=new Dictionary<string,string>{{"F1","index"},{"F2","bank"},{"F3","macro-search"},{"F4","phase-input"},{"F5","filter"},{"F7","local"},{"F8","global"},{"F9","keyboard"},{"F10","worksheet"},{"F12","review"},{"Control+Shift+KeyN","next-pin"},{"Control+Alt+KeyN","next-activate"},{"Control+Shift+KeyL","next-locate"},{"Control+Shift+Backspace","next-clear"},{"Alt+Digit1","bank-A"},{"Alt+Digit2","bank-B"},{"Alt+Digit3","bank-I"},{"Alt+Digit4","bank-M"},{"Alt+Digit5","bank-E"}};

 internal ExperimentShell(Form form,string exe,string root,string token,string mode){
  this.form=form;this.mode=mode;api=new LocalApi(root,token);
  bridge=new ExperimentBridge(form,exe,delegate{return IsReady&&!closing;});
  RegisterCommands();Build();
  input=new ExperimentInput(form,f=>f==form||windows.Contains(f),InputState,
   cap=>DrawGripFeedback(),turn=>Send(LocalApi.D("action","twist","cell",turn.Cell,"axis",turn.Axis,"inverse",turn.Inverse,"destination",turn.Destination,"phase",turn.Phase,"state_hash",work["hash"],"bank",Workspace["bank"],"ordered_vertices",Value(Map(Value(Bank,"frames")),turn.Cell.ToString()))),
   RunKeyboardCommand,text=>{inputFeedback.Text=text;DrawGripFeedback();});
  input.VisualChanged=DrawInputPressFeedback;
  input.Diagnostic=line=>NativeDiagnostics.Write("Input "+line);
  defaultKeys["F6"]="compare-preview";
  defaultKeys["Shift+F2"]="orbit";
  defaultKeys["Control+Digit0"]="inspect-actual";
  defaultKeys["Shift+Escape"]="cancel-analysis";
  defaultKeys["Control+Digit1"]="phase-prepare";defaultKeys["Control+Digit2"]="phase-macro";defaultKeys["Control+Digit3"]="phase-cleanup";
  defaultKeys["Control+KeyM"]="macro-insert";defaultKeys["Control+Shift+KeyI"]="cleanup-inverse";
  defaultKeys["Control+KeyZ"]="undo";defaultKeys["Control+Shift+KeyZ"]="redo";
  defaultKeys["Control+KeyR"]="operation-focus";defaultKeys["Shift+F12"]="review-details";
  defaultKeys["Control+F1"]="solve-actions";
  defaultKeys["Shift+F1"]="help";
  defaultKeys["Control+Alt+Digit1"]="bank-macro-1";defaultKeys["Control+Alt+Digit2"]="bank-macro-2";
  defaultKeys["Control+Alt+Digit4"]="effect-body";defaultKeys["Control+Alt+Digit5"]="effect-complete";
  defaultKeys["Control+Shift+KeyC"]="set-current";defaultKeys["Control+Shift+KeyA"]="assign-a";defaultKeys["Control+Shift+KeyB"]="assign-b";defaultKeys["Control+Shift+KeyT"]="assign-target";defaultKeys["Control+Shift+KeyP"]="block-protect";defaultKeys["Control+Shift+KeyH"]="block-add";
  input.Attach();Application.AddMessageFilter(this);
  bridge.Turn+=tokens=>NativeInput(LocalApi.D("action","turn","tokens",tokens,"destination",Text(destination),"phase",Text(phase)));
  bridge.Inspect+=(slot,gesture)=>NativeInput(LocalApi.D("action","inspect","native_sticker",slot,"gesture",gesture));
  hub.ObjectSelected+=(kind,id)=>Send(LocalApi.D("action",kind=="identity"?"inspect":"inspect-position",kind=="identity"?"identity":"position",id));
  hub.ActionRequested+=HubAction;
  hub.PhaseRequested+=boundary=>RunCommand("inspect-"+boundary);
  hub.EffectScopeRequested+=scope=>RunCommand(scope=="body"?"effect-body":"effect-complete");
  hub.KeyLabel=id=>KeyHint(id=="pin-next"?"next-pin":id=="protect-position"?"block-protect":id);hub.OrbitLabel=OrbitName;hub.CellLabel=CellName;local.CellLabel=CellName;global.CellLabel=CellName;hub.CommandRequested+=RunCommand;
  local.SelectionChanged+=SelectCell;global.SelectionChanged+=SelectCell;
  local.CenterRequested+=bridge.LocateCell;global.CenterRequested+=bridge.LocateCell;
  form.Shown+=delegate{
   // Apply the experiment's minimum after the retained Form has completed loading.
   form.MinimumSize=new Size(1000,650);
   if(form.WindowState==FormWindowState.Normal){var area=Screen.FromControl(form).WorkingArea;int width=Math.Min(form.Width,area.Width),height=Math.Min(form.Height,area.Height);form.Bounds=new Rectangle(Math.Max(area.Left,Math.Min(form.Left,area.Right-width)),Math.Max(area.Top,Math.Min(form.Top,area.Bottom-height)),width,height);}
   FitWorkspace();
   // Choose a persistent non-text entry before loading; never reclaim focus when loading finishes.
   if(mode=="g2"&&Form.ActiveForm==form&&orbit.Focused&&!orbit.DroppedDown){var first=commandButtons.FirstOrDefault(p=>p.Value=="keyboard"&&p.Key.CanFocus).Key;if(first!=null)first.Focus();}
   Connect();
  };
  form.FormClosed+=delegate{closing=true;Dispose();};
 }
 internal Form Window {get{return form;}}
 internal bool IsReady {get{return connected&&!OperationInputBlocked&&!closing;}}
 internal Dictionary<string,object> Work {get{return work;}}
 internal Control PuzzleViewport {get{return bridge.Viewport;}}
 internal bool Recover(Exception error){return bridge.Recover(error);}
 static Dictionary<string,object> Map(object value){return value as Dictionary<string,object>;}
 static object Value(Dictionary<string,object> value,string key){object result;return value!=null&&value.TryGetValue(key,out result)?result:null;}
 static object[] Items(object value){return value as object[]??new object[0];}
 static string Text(object value){var control=value as Control;return control!=null?control.Text:Convert.ToString(value);}
 static int Number(object value){return Convert.ToInt32(value);}
 Dictionary<string,object> Workspace {get{return Map(Value(work,"workspace"));}}
 Dictionary<string,object> Selection {get{return Map(Value(work,"inspected"))??Map(Value(work,"current"));}}
 Dictionary<string,object> Bank {get{return Items(Value(work,"banks")).Select(Map).FirstOrDefault(b=>Text(b["id"])==Text(Value(Workspace,"bank")));}}
 int SelectedIdentity(){if(Selection==null)throw new InvalidOperationException("Inspect a physical piece first.");return Number(Selection["piece"]);}
 int SelectedPosition(){if(Selection==null)throw new InvalidOperationException("Inspect a fixed position first.");return Number(Selection["position"]);}
 void Say(string text,bool error=false){feedback.Text=text;feedback.AccessibleName="Status: "+text;feedback.ForeColor=error?Color.FromArgb(238,141,135):Ink;tips.SetToolTip(feedback,text);NativeDiagnostics.Write("Experiment status: "+text);}
 void Register(string id,string label,Action action){commands.Add(id,action);hints.Add(id,label);}
 void RunCommand(string id){try{if(closing)return;Action action;if(!commands.TryGetValue(id,out action))throw new InvalidOperationException("Unknown command: "+id);if(OperationInputBlocked&&!new[]{"cancel-analysis","macro-search","macro-close","solve-macros","solve-prepare","solve-protection","index","bank","keyboard","keyboard-extra","views-close","local","global","puzzle","operation-focus","help"}.Contains(id))throw new InvalidOperationException("Finish or stop the current check before changing the operation. No input was queued.");if(hub.SelectionIsForecast&&forecastRestrictedCommands.Contains(id))throw new InvalidOperationException("This is a forecast identity. Select an actual token or a fixed position before assigning, capturing or protecting its current state.");action();}catch(Exception e){Say(e.Message,true);NativeDiagnostics.Write("Command rejected: "+id,e);}}
 void RunKeyboardCommand(string id){if(id=="preview"||id=="commit"){var b=FocusedOwnedControl() as Button;string command;if(b==null||b.Parent!=operationStrip||!commandButtons.TryGetValue(b,out command)||!new[]{"review","preview","commit","cancel-preview"}.Contains(command)){Say("Focus Operation controls first"+KeyHint("operation-focus")+".",true);return;}}RunCommand(id);}
 Button Button(string label,string command,Button supplied=null){
  var b=supplied??new Button();UseRoundedButton(b);b.Text=label;b.Tag=label;b.AutoSize=true;b.AutoSizeMode=AutoSizeMode.GrowAndShrink;b.MinimumSize=new Size(0,29);b.Margin=new Padding(2);b.Padding=new Padding(5,1,5,1);b.FlatStyle=FlatStyle.Flat;b.BackColor=Paper;b.ForeColor=Ink;b.AccessibleName=hints.ContainsKey(command)?hints[command]:label;
  b.FlatAppearance.BorderSize=0;b.ForeColor=Ink;b.BackColor=Paper;b.FlatAppearance.MouseOverBackColor=Color.FromArgb(34,45,56);b.Click+=delegate{RunCommand(command);};var menu=new ContextMenuStrip();menu.Items.Add("Change key…",null,delegate{ShowBindingEditor(command);});b.ContextMenuStrip=menu;b.Disposed+=delegate{menu.Dispose();};tips.SetToolTip(b,b.AccessibleName+" · right-click to change key");commandButtons[b]=command;return b;
 }

 FlowLayoutPanel Strip(){var row=new FlowLayoutPanel{Dock=DockStyle.Fill,WrapContents=false,Margin=Padding.Empty,Padding=new Padding(4,1,4,1)};strips.Add(row);return row;}
 static FlowLayoutPanel Row(){return new FlowLayoutPanel{AutoSize=true,Dock=DockStyle.Top,WrapContents=true,Margin=Padding.Empty,Padding=new Padding(4,2,4,2)};}
 static void Combo(ComboBox box,params string[] values){box.DropDownStyle=ComboBoxStyle.DropDownList;box.Items.AddRange(values);box.SelectedIndex=0;box.Width=100;box.Margin=new Padding(3);}
 internal static SplitContainer CreateSplit(int width,int distance,int leftMin,int rightMin){return new SplitContainer{Size=new Size(width,450),SplitterDistance=distance,Dock=DockStyle.Fill,SplitterWidth=5,Panel1MinSize=leftMin,Panel2MinSize=rightMin};}
 void BuildGripReview(){
  form.SuspendLayout();
  try{
   form.Text="Magic 600 Cell · "+mode.ToUpperInvariant()+" native experiment · post-approval development";
   form.Font=new Font("Segoe UI",10f);form.BackColor=Paper;form.ForeColor=Ink;
   form.MinimumSize=new Size(1000,650);form.Size=new Size(1280,800);form.KeyPreview=false;
   foreach(string name in new[]{"panel1","panel2","splitter1","menuStrip1","panel3","splitter2"}){var c=Reflect.Get(form,name) as Control;if(c!=null)c.Hide();}
   var root=new TableLayoutPanel{Size=form.ClientSize,Dock=DockStyle.Fill,ColumnCount=1,RowCount=7,Margin=Padding.Empty,Padding=Padding.Empty,BackColor=form.BackColor};layoutRoot=root;
   root.ColumnStyles.Add(new ColumnStyle(SizeType.Percent,100));
   foreach(int height in new[]{32,30,0,0,34,32,24})root.RowStyles.Add(new RowStyle(SizeType.Absolute,height));
   root.RowStyles[2]=new RowStyle(SizeType.Percent,100);
   var header=Strip();orbit.DropDownStyle=ComboBoxStyle.DropDownList;orbit.Width=245;orbit.AccessibleName="Active orbit by mathematical structure";
   orbit.SelectionChangeCommitted+=delegate{if(!updating)Send(LocalApi.D("action","orbit","orbit",Int32.Parse(((Choice)orbit.SelectedItem).Id)));};
   header.Controls.Add(orbit);header.Controls.Add(Button("Macro Base","macro-search"));header.Controls.Add(Button("Solve actions","solve-actions"));header.Controls.Add(Button("Views…","views"));header.Controls.Add(Button("Commands","index"));header.Controls.Add(Button("Keymap","bank"));
   context.AutoSize=true;context.Padding=new Padding(6,5,4,4);if(mode=="g1")header.Controls.Add(context);root.Controls.Add(header,0,0);
   var boundary=Strip();protection.AutoSize=true;protection.Padding=new Padding(3,3,5,0);boundary.Controls.Add(protection);strict.Text="Protect each step";strict.AutoSize=true;strict.CheckedChanged+=delegate{if(!updating)RunCommand(strict.Checked?"prefix-on":"prefix-off");};boundary.Controls.Add(strict);boundary.Controls.Add(Button("Protection…","protection"));boundary.Controls.Add(Button("Check results","review-details"));root.Controls.Add(boundary,0,1);
   catalogue=new TableLayoutPanel{Dock=DockStyle.Fill,ColumnCount=2,RowCount=2,Padding=new Padding(4),Margin=Padding.Empty,Visible=false};
   catalogue.ColumnStyles.Add(new ColumnStyle(SizeType.Percent,60));catalogue.ColumnStyles.Add(new ColumnStyle(SizeType.Percent,40));
   catalogue.RowStyles.Add(new RowStyle(SizeType.Absolute,28));catalogue.RowStyles.Add(new RowStyle(SizeType.Percent,100));
   search.Dock=DockStyle.Fill;search.AccessibleName="Macro Base search";search.TextChanged+=delegate{DrawMacros();};macros.Dock=DockStyle.Fill;macros.IntegralHeight=false;macros.AccessibleName="Existing Macro Base entries";
   search.KeyDown+=delegate(object sender,KeyEventArgs e){if(e.KeyCode==Keys.Escape){RunCommand("macro-close");e.Handled=true;e.SuppressKeyPress=true;}};
   var searchRow=new Panel{Dock=DockStyle.Fill};var closeCatalogue=new Button{Text="×",Dock=DockStyle.Right,Width=32,AccessibleName="Close Macro Base and return to work"};closeCatalogue.Click+=delegate{RunCommand("macro-close");};tips.SetToolTip(closeCatalogue,"Return to the graphical workspace · Escape in search");searchRow.Controls.Add(search);searchRow.Controls.Add(closeCatalogue);
   macros.SelectedIndexChanged+=delegate{if(!updating&&macros.SelectedItem!=null){if(busy){DrawMacros();return;}selectedMacro=((Choice)macros.SelectedItem).Id;selectedEffect=null;Send(LocalApi.D("action","macro-effect","id",selectedMacro,"select",true));}};
   var macroActions=Strip();macroActions.Controls.Add(Button("Append macro","macro-insert"));macroActions.Controls.Add(Button("New…","macro-new"));macroActions.Controls.Add(Button("Details…","macro-details"));
   effectText.Dock=DockStyle.Fill;effectText.Padding=new Padding(3);effectText.Text="Select an existing macro to inspect its exact effect.";
   catalogue.Controls.Add(searchRow,0,0);catalogue.Controls.Add(macros,0,1);catalogue.Controls.Add(macroActions,1,0);catalogue.Controls.Add(effectText,1,1);root.Controls.Add(catalogue,0,3);
   var workspace=CreateSplit(850,560,340,240);viewSplit=workspace;
   hub.Dock=DockStyle.Fill;workspace.Panel1.Controls.Add(hub);
   views.Dock=DockStyle.Fill;views.AccessibleName="Native puzzle and structural views";
   var puzzlePage=new TabPage("Puzzle");var localPage=new TabPage("Local");var globalPage=new TabPage("Global");views.TabPages.AddRange(new[]{puzzlePage,localPage,globalPage});
   bridge.Viewport.Parent=puzzlePage;bridge.Viewport.Dock=DockStyle.Fill;bridge.Viewport.Visible=true;
   var puzzleDisplay=Button("Display…","display");puzzleDisplay.Dock=DockStyle.Bottom;puzzlePage.Controls.Add(puzzleDisplay);puzzleDisplay.BringToFront();
   local.Dock=DockStyle.Fill;global.Dock=DockStyle.Fill;localPage.Controls.Add(local);globalPage.Controls.Add(global);
   views.SelectedIndexChanged+=delegate{local.SetRenderingActive(views.SelectedIndex==1);global.SetRenderingActive(views.SelectedIndex==2);};
   var contextPanel=new TableLayoutPanel{Dock=DockStyle.Fill,RowCount=2,ColumnCount=1};contextPanel.RowStyles.Add(new RowStyle(SizeType.Percent,100));contextPanel.RowStyles.Add(new RowStyle(SizeType.Absolute,90));
   pieceText.Dock=DockStyle.Fill;pieceText.Padding=new Padding(8);contextPanel.Controls.Add(views,0,0);contextPanel.Controls.Add(pieceText,0,1);workspace.Panel2.Controls.Add(contextPanel);
   if(mode=="g1"){workspace.Panel1Collapsed=true;pieceText.Text="Choose a piece, then capture explicit caps. Turn input is disabled until the complete native mapping passes.";}
   workspace.Margin=Padding.Empty;root.Controls.Add(workspace,0,2);
   var operation=Strip();operationStrip=operation;foreach(string p in new[]{"prepare","macro","cleanup"}){var b=Button(p,"phase-"+p);phaseButtons[p]=b;operation.Controls.Add(b);}
   operation.Controls.Add(Button("New operation","operation-new"));operation.Controls.Add(Button("Reuse steps","operation-reuse"));operation.Controls.Add(Button("Edit steps","phase-input"));operation.Controls.Add(Button("Review","review"));operation.Controls.Add(Button("Preview","preview"));operation.Controls.Add(Button("Execute","commit"));operation.Controls.Add(Button("Cancel preview","cancel-preview"));operation.Controls.Add(Button("Stop check","cancel-analysis"));root.Controls.Add(operation,0,4);
   var inputRow=Strip();Combo(destination,"draft","live");Combo(phase,"prepare","macro","cleanup");Combo(gripMode,"hold","latch");
   destination.SelectionChangeCommitted+=delegate{if(!updating)RunCommand("input-"+Text(destination));};
   gripMode.SelectionChangeCommitted+=delegate{if(!updating&&input!=null)RunCommand("grip-"+Text(gripMode));};
   inputRow.Controls.Add(new Label{Text="Turns to",AutoSize=true,Padding=new Padding(5,4,2,0)});inputRow.Controls.Add(destination);if(mode=="g1")inputRow.Controls.Add(gripMode);inputRow.Controls.Add(Button("Keyboard","keyboard"));if(mode=="g1")inputRow.Controls.Add(Button("Capture / rebind…","capture"));
   bankLabel.AutoSize=true;bankLabel.Padding=new Padding(5,5,4,4);inputRow.Controls.Add(bankLabel);
   keyboard.Dock=DockStyle.Fill;keyboard.AutoSize=false;keyboard.AutoScroll=true;keyboard.WrapContents=true;keyboard.Padding=new Padding(5);
   root.Controls.Add(inputRow,0,5);
   var status=new TableLayoutPanel{Dock=DockStyle.Fill,ColumnCount=2,RowCount=1,Margin=Padding.Empty};status.ColumnStyles.Add(new ColumnStyle(SizeType.Percent,65));status.ColumnStyles.Add(new ColumnStyle(SizeType.Percent,35));
   inputFeedback.Dock=DockStyle.Fill;inputFeedback.AutoEllipsis=true;inputFeedback.Padding=new Padding(5,2,5,0);
   feedback.Dock=DockStyle.Fill;feedback.AutoEllipsis=true;feedback.Padding=new Padding(5,2,5,0);feedback.AccessibleName="Operation result and error";feedback.Text="Connecting to the isolated native engine…";status.Controls.Add(feedback,0,0);status.Controls.Add(inputFeedback,1,0);root.Controls.Add(status,0,6);
   form.Controls.Add(root);root.BringToFront();
   form.ClientSizeChanged+=delegate{if(form.Visible)FitWorkspace();};
  }finally{form.ResumeLayout(true);}
 }
 void FitWorkspace(){
  if(mode=="g2"){FitGraphWorkspace();return;}
  if(layoutRoot==null)return;layoutRoot.SuspendLayout();
  try{
   int h=Math.Max(30,form.Font.Height+13);
   layoutRoot.RowStyles[0].Height=h;layoutRoot.RowStyles[1].Height=h-2;layoutRoot.RowStyles[3].Height=catalogue.Visible?h*4:0;catalogue.RowStyles[0].Height=h;
   layoutRoot.RowStyles[4].Height=h+2;layoutRoot.RowStyles[5].Height=h;layoutRoot.RowStyles[6].Height=form.Font.Height+5;
   foreach(var row in strips){row.Padding=new Padding(4,1,4,1);foreach(Control c in row.Controls){c.Margin=new Padding(1);var b=c as Button;if(b!=null)b.MinimumSize=new Size(0,h-4);}}
   orbit.Width=245;destination.Width=76;gripMode.Width=90;
  }finally{layoutRoot.ResumeLayout(true);}
 }

 sealed class Choice {internal string Id,Label;internal Choice(string id,string label){Id=id;Label=label;}public override string ToString(){return Label;}}
 string OrbitName(int id){var r=Items(Value(structure,"orbit_profiles")).Select(Map).FirstOrDefault(x=>Number(x["orbit"])==id);var names=Map(Value(r,"names"));return names!=null?Text(Value(names,"name")):r==null?"Orbit "+id:Text(r["hosting_count"])+"-cell · "+r["affecting_cap_count"]+"-cap · "+(Text(r["orientation_group"])=="trivial"?"no orientation":r["orientation_group"]+" orientation");}
 void Connect(){
  // The retained DirectX device must initialize in a visible viewport before it can be hidden.
  if(mode=="g2"){viewSplit.Panel2Collapsed=false;views.SelectedIndex=0;}
  busy=true;Dictionary<string,object> profile=null,geometry=null,reply=null;NativeSnapshot next=null;
  Task.Factory.StartNew(delegate{
   NativeDiagnostics.Write("Experiment: exporting native geometry");geometry=bridge.Geometry();api.Post("native/handshake",geometry);profile=api.Get("native/map");structure=api.Get("structure");reply=api.Get("experiment/native-snapshot");next=ReadPair(reply,profile,null);NativeDiagnostics.Write("Experiment: coherent native/context snapshot received");
  }).ContinueWith(t=>OnUi(delegate{
   if(t.IsFaulted){busy=false;Say(t.Exception.GetBaseException().Message,true);NativeDiagnostics.Write("Experiment connection failed",t.Exception.GetBaseException());return;}
   try{
    NativeDiagnostics.Write("Experiment: configuring native display");bridge.Configure(profile);bridge.Apply(next);bridge.Ready();snapshot=next;work=Map(reply["work"]);
    NativeDiagnostics.Write("Experiment: parsing structural geometry");var g=NativeCellGeometry.FromStructure(structure,bridge.Palette);local.Configure(g);global.Configure(g);hub.SetGeometry(g);local.RegistryCameraInput=true;global.RegistryCameraInput=true;if(Value(structure,"local_geometry")!=null)local.ConfigureStickers(Map(structure["local_geometry"]));lastLocalCell=Map(Value(reply,"local_cell"));if(lastLocalCell!=null)local.UpdateCellState(lastLocalCell);
    connected=true;AdoptMacroSelection();NativeDiagnostics.Write("Experiment: binding workspace controls");Draw();Say("Native mapping verified: all 259,800 slots and 1,200 generators. Choose your work explicitly.");
   }
   catch(Exception e){connected=false;Say(e.Message,true);NativeDiagnostics.Write("Experiment display initialization failed",e);}
   finally{busy=false;bridge.SetBusy(!connected);input.RefreshFocusFeedback();RestoreWorkWindows();if(connected&&mode=="g2")Send(LocalApi.D("action","inspect-phase"));}
  }));
 }
 NativeSnapshot ReadPair(Dictionary<string,object> reply,Dictionary<string,object> profile,NativeSnapshot previous){
  var native=NativeSnapshot.Read(Map(reply["native_snapshot"]),Text(profile["profile_sha256"]),previous);var state=Map(reply["work"]);
  if(Text(state["hash"])!=Text(native.State["state_hash"])||Text(state["head"])!=Text(native.State["head"]))throw new InvalidDataException("Hub context and native snapshot disagree; input remains disabled.");
  var context=Map(Value(state,"review_context"));
  if(context==null||Text(Value(context,"id")).Length==0||Text(Value(context,"model"))!=Text(Value(state,"model"))||Text(Value(context,"state_hash"))!=Text(Value(state,"hash"))||Text(Value(context,"head"))!=Text(Value(state,"head"))||Text(Value(context,"revision"))!=Text(Value(state,"revision"))||Text(Value(context,"epoch"))!=Text(Value(state,"epoch")))throw new InvalidDataException("Analysis context and committed state disagree; input remains disabled.");
  return native;
 }
 void OnUi(Action callback,Action skipped=null){if(closing||form.IsDisposed||!form.IsHandleCreated){if(skipped!=null)skipped();return;}try{form.BeginInvoke((Action)delegate{if(!closing&&!form.IsDisposed)callback();else if(skipped!=null)skipped();});}catch(ObjectDisposedException){if(skipped!=null)skipped();}catch(InvalidOperationException){if(skipped!=null)skipped();}}
 // Opt-in diagnostic only. Record intervals, never request payloads or labels.
 sealed class SendTiming {
  internal readonly long Start=Stopwatch.GetTimestamp();internal readonly int Id;internal readonly string Action;
  internal long Submitted,WorkerStart,PostDone,WorkerDone,UiQueued,UiStart,ApplyStart,ApplyDone,DrawStart,HubStart,HubDone,DrawDone,FinallyStart,AvailabilityDone,KeyboardDone,Completed,KeyHintCalls,KeyHintTicks;
  internal bool Refresh;
  internal long CancelDone,PhaseDone,HubDisabled,KeyHintEndCalls,KeyHintEndTicks,AvailabilityHubDone,AvailabilityBridgeDone,AvailabilityInputDone;
  internal SendTiming(int id,string action){Id=id;Action=action;}
 }
 static double? SendTimingMs(long start,long end){return start==0||end==0?(double?)null:(end-start)*1000.0/Stopwatch.Frequency;}
 void CompleteSendTiming(SendTiming t){if(t==null)return;t.Completed=Stopwatch.GetTimestamp();t.KeyHintEndCalls=sendTimingKeyHintCalls;t.KeyHintEndTicks=sendTimingKeyHintTicks;}
 void WriteSendTiming(SendTiming t,bool? accepted,string outcome){
  if(t==null)return;
  try{NativeDiagnostics.Write("Experiment send timing: "+api.Json(LocalApi.D(
   "request",t.Id,"action",t.Action,"outcome",outcome,"accepted",accepted,"refresh",t.Refresh,
   "entry_to_submit_ms",SendTimingMs(t.Start,t.Submitted),"submit_to_worker_ms",SendTimingMs(t.Submitted,t.WorkerStart),
   "post_ms",SendTimingMs(t.WorkerStart,t.PostDone),"post_to_worker_done_ms",SendTimingMs(t.PostDone,t.WorkerDone),"worker_total_ms",SendTimingMs(t.WorkerStart,t.WorkerDone),
   "worker_to_ui_queue_ms",SendTimingMs(t.WorkerDone,t.UiQueued),"ui_queue_ms",SendTimingMs(t.UiQueued,t.UiStart),"ui_before_apply_ms",SendTimingMs(t.UiStart,t.ApplyStart),
   "apply_ms",SendTimingMs(t.ApplyStart,t.ApplyDone),"apply_to_draw_ms",SendTimingMs(t.ApplyDone,t.DrawStart),
   "draw_ms",SendTimingMs(t.DrawStart,t.DrawDone),"hub_ms",SendTimingMs(t.HubStart,t.HubDone),"draw_other_ms",SendTimingMs(t.DrawStart,t.DrawDone)-SendTimingMs(t.HubStart,t.HubDone),
   "draw_to_finally_ms",SendTimingMs(t.DrawDone,t.FinallyStart),"availability_ms",SendTimingMs(t.FinallyStart,t.AvailabilityDone),"keyboard_end_ms",SendTimingMs(t.AvailabilityDone,t.KeyboardDone),
   "finally_tail_ms",SendTimingMs(t.KeyboardDone,t.Completed),"total_ms",SendTimingMs(t.Start,t.Completed),
   "cancel_pending_ms",SendTimingMs(t.Start,t.CancelDone),"prepare_phase_ms",SendTimingMs(t.CancelDone,t.PhaseDone),"disable_hub_ms",SendTimingMs(t.PhaseDone,t.HubDisabled),"disable_rest_ms",SendTimingMs(t.HubDisabled,t.Submitted),
   "availability_hub_ms",SendTimingMs(t.FinallyStart,t.AvailabilityHubDone),"availability_bridge_ms",SendTimingMs(t.AvailabilityHubDone,t.AvailabilityBridgeDone),"availability_input_ms",SendTimingMs(t.AvailabilityBridgeDone,t.AvailabilityInputDone),"availability_rest_ms",SendTimingMs(t.AvailabilityInputDone,t.AvailabilityDone),
   "key_hint_calls",t.KeyHintEndCalls-t.KeyHintCalls,"key_hint_ms",(t.KeyHintEndTicks-t.KeyHintTicks)*1000.0/Stopwatch.Frequency)));}
  catch { /* Diagnostics must not change the request result. */ }
 }
 internal Task<bool> Send(Dictionary<string,object> body){return SendRoute("experiment/native-command",body);}
 Task<bool> SendRoute(string route,Dictionary<string,object> body){
  var completion=new TaskCompletionSource<bool>();var timing=sendTimingEnabled?new SendTiming(++sendTimingSequence,Text(Value(body,"action"))):null;
  if(timing!=null){timing.KeyHintCalls=sendTimingKeyHintCalls;timing.KeyHintTicks=sendTimingKeyHintTicks;}
  if(!connected||OperationInputBlocked||closing){hub.CancelPendingInteraction();if(!closing&&work!=null&&route=="experiment/native-command"&&Text(Value(body,"action"))=="macro-effect"&&Object.Equals(Value(body,"select"),true)){AdoptMacroSelection();Draw();}Say(stopFailure??(stopPending?"Waiting for stop acknowledgement; no operation was queued.":"Input was not accepted: the native bridge is not ready or an operation is busy."),true);CompleteSendTiming(timing);completion.SetResult(false);WriteSendTiming(timing,false,"rejected");return completion.Task;}
  string requestedAction=Text(Value(body,"action"));
  bool isImport=requestedAction=="session-log-apply";
  string previousHead=Text(Value(work,"head"));
  NativeDiagnostics.Write("Experiment dispatch: "+route+" / "+requestedAction);
  if(route!="experiment/native-command"||requestedAction!="inspect"&&requestedAction!="inspect-position")hub.CancelPendingInteraction();
  if(timing!=null)timing.CancelDone=Stopwatch.GetTimestamp();long phaseSerial=PreparePhaseRequest(body);if(timing!=null)timing.PhaseDone=Stopwatch.GetTimestamp();
  busy=true;hub.SetInteractionEnabled(false);if(timing!=null)timing.HubDisabled=Stopwatch.GetTimestamp();macros.Enabled=false;workGoal.Enabled=false;bridge.SetBusy(true);body["native_since"]=snapshot.Revision;Say("Checking the explicit operation; committed state remains visible.");
  Dictionary<string,object> reply=null,receipt=null;NativeSnapshot next=null;Exception fault=null;
  if(timing!=null)timing.Submitted=Stopwatch.GetTimestamp();
  Task.Factory.StartNew(delegate{
   if(timing!=null)timing.WorkerStart=Stopwatch.GetTimestamp();
   try{var outcome=ExperimentSendOutcome.Worker<NativeSnapshot>(delegate{var posted=api.Post(route,body);if(timing!=null)timing.PostDone=Stopwatch.GetTimestamp();return posted;},delegate{if(timing!=null)timing.Refresh=true;return api.Get("experiment/native-snapshot");},(fresh,useBase)=>ReadPair(fresh,bridge.Profile,useBase?snapshot:null),requestedAction,refresh=>NativeDiagnostics.Write("Native error recovery snapshot failed",refresh));reply=outcome.Reply;receipt=outcome.Receipt;next=outcome.Next;fault=outcome.Fault;}
   finally{if(timing!=null)timing.WorkerDone=Stopwatch.GetTimestamp();}
  }).ContinueWith(t=>{if(t.IsFaulted)fault=t.Exception.GetBaseException();if(timing!=null)timing.UiQueued=Stopwatch.GetTimestamp();OnUi(delegate{
   if(timing!=null)timing.UiStart=Stopwatch.GetTimestamp();
   BeginKeyboardBatch();
   try{
    if(t.IsFaulted)fault=t.Exception.GetBaseException();
    if(fault!=null)hub.CancelPendingInteraction();
    if(next!=null){if(timing!=null)timing.ApplyStart=Stopwatch.GetTimestamp();bridge.Apply(next);if(timing!=null)timing.ApplyDone=Stopwatch.GetTimestamp();snapshot=next;work=Map(reply["work"]);AdoptMacroSelection();lastLocalCell=Map(Value(reply,"local_cell"));if(lastLocalCell!=null)local.UpdateCellState(lastLocalCell);var result=Map(Value(reply,"result"));string action=Text(Value(body,"action"));selectedApplicability=null;if(action=="macro-effect"&&fault==null&&result!=null&&Text(Value(body,"id"))==selectedMacro){selectedEffect=Map(Value(result,"effect"));selectedEffectKey=SelectedMacroEffectKey();selectedEffectScope="body";selectedApplicability=Map(Value(result,"applicability"));}else if(action=="review")selectedEffectScope="complete";bool? requestedPreview=action=="preview"&&fault==null?(bool?)true:action=="commit"||action=="cancel-preview"?(bool?)false:null;AdoptPhaseInspection(reply,phaseSerial);DrawTimed(requestedPreview,timing);}
    if(fault!=null)Say(fault.Message,true);
    else if(Text(Value(Map(Value(reply,"result")),"warning")).Length>0){var outcome=Map(Value(reply,"result"));string warning=Text(Value(outcome,"warning"));bool committed=Object.Equals(Value(outcome,"committed"),true);Say((isImport&&receipt!=null?"Log imported. ":committed?"Operation committed. ":"Workspace saved. ")+warning,true);NativeDiagnostics.Write("Successful write follow-up warning: "+warning);}
    else if(requestedAction=="inspect-phase"&&mode=="g2"&&phaseInspection==null)Say("Draft inspection did not complete. "+Text(Value(reply,"phase_inspection_error")),true);
    else Say("Completed explicit "+Text(body["action"])+"."+(Text(Value(reply,"phase_inspection_error")).Length>0?" Draft inspection unavailable; actual state is retained.":""));
    if(next==null){connected=false;Say("Could not recover a complete native snapshot; input disabled. Relaunch this isolated session.",true);}
   }catch(Exception e){connected=false;Say(e.Message,true);NativeDiagnostics.Write("Experimental completion failed",e);}
   finally{
    if(timing!=null)timing.FinallyStart=Stopwatch.GetTimestamp();
    var finalized=ExperimentSendOutcome.Finalize(receipt,isImport,Map(Value(reply,"result")),fault,connected,requestedAction!="inspect-phase"||mode!="g2"||phaseInspection!=null);busy=false;
    try{
     try{
      if(finalized.Accepted&&requestedAction=="session-new")pendingDefaultViews=true;
      RefreshCommandAvailabilityTimed(timing);
     }finally{if(timing!=null)timing.AvailabilityDone=Stopwatch.GetTimestamp();try{EndKeyboardBatch();}finally{if(timing!=null)timing.KeyboardDone=Stopwatch.GetTimestamp();}}
    }catch(Exception e){connected=false;bridge.SetBusy(true);hub.SetInteractionEnabled(false);macros.Enabled=false;workGoal.Enabled=false;Say(e.Message,true);NativeDiagnostics.Write("Experimental keyboard finalization failed",e);}
    finally{
     try{
      finalized=ExperimentSendOutcome.Finalize(receipt,isImport,Map(Value(reply,"result")),fault,connected,requestedAction!="inspect-phase"||mode!="g2"||phaseInspection!=null);
      lastCommandResult=finalized.LastResult;
      if(finalized.ReceiptWarning!=null){receipt["warning"]=Text(Value(receipt,"warning"))+" "+finalized.ReceiptWarning;Say(finalized.ReceiptWarning,true);}
      if(finalized.LogRejection)NativeDiagnostics.Write("Experimental operation rejected",fault);
      if(finalized.Accepted&&previousHead!=Text(Value(work,"head"))&&(requestedAction=="commit"||requestedAction=="twist"||requestedAction=="native-word"||route=="experiment/native-input"))RequestSessionCompletion();
     }finally{CompleteSendTiming(timing);completion.TrySetResult(finalized.Accepted);WriteSendTiming(timing,finalized.Accepted,"completed");}
    }
   }
  },()=>{CompleteSendTiming(timing);completion.TrySetCanceled();WriteSendTiming(timing,null,"ui-skipped");});});return completion.Task;
 }
 void NativeInput(Dictionary<string,object> body){if(!IsReady)return;body["profile_sha256"]=bridge.Profile["profile_sha256"];body["state_hash"]=work["hash"];SendRoute("experiment/native-input",body);}
 void AdoptMacroSelection(){
  var saved=Map(Value(Workspace,"selected_macro"));if(saved!=null&&!CurrentMacroBinding(saved))saved=null;
  string id=saved==null?null:Text(saved["id"]),binding=saved==null?null:api.Json(BoundMacro(saved));int orbitId=Number(Workspace["orbit"]);
  if(macroWorkOrbit!=orbitId||macroWorkBinding!=binding||selectedMacro!=id){selectedMacro=id;selectedEffect=null;selectedEffectKey=null;selectedApplicability=null;selectedEffectScope="body";}
  macroWorkOrbit=orbitId;macroWorkBinding=binding;
 }
 void Draw(bool? requestedPreview=null){DrawTimed(requestedPreview,null);}
 void DrawTimed(bool? requestedPreview,SendTiming timing){
  if(work==null)return;if(timing!=null)timing.DrawStart=Stopwatch.GetTimestamp();updating=true;
  try{

   NativeDiagnostics.Write("Experiment draw: orbit catalogue");
   if(orbit.Items.Count==0){orbit.BeginUpdate();try{foreach(var p in Items(Value(structure,"orbit_profiles"))){int id=Number(Map(p)["orbit"]);orbit.Items.Add(new Choice(id.ToString(),OrbitName(id)));}}finally{orbit.EndUpdate();}}
   for(int i=0;i<orbit.Items.Count;i++)if(((Choice)orbit.Items[i]).Id==Text(Workspace["orbit"]))orbit.SelectedIndex=i;
   var current=Map(work["current"]);var next=Map(work["next"]);
   context.Text="Current "+PieceCaption(current)+"    Next "+(next==null?"unassigned":PieceCaption(next)+" [bookmark]");
   var review=Map(work["review"]);var locks=Items(Value(work,"position_locks")??Map(Workspace["block"])["protected"]).Select(Map).ToArray();protection.Text="Protected: "+Items(work["protected"]).Length+" orbits · "+locks.Select(x=>Number(x["position"])).Distinct().Count()+" positions  |  "+(review==null?"Not checked":Text(review["status"]));tips.SetToolTip(protection,String.Join("\n",locks.Select(x=>"P"+x["position"]+" · "+OrbitName(Number(Value(x,"orbit")??Workspace["orbit"]))+" · captured labels "+String.Join(", ",Items(x["labels"]).Select(Text)))));
   strict.Checked=Convert.ToBoolean(Workspace["prefix"]);destination.SelectedItem=Text(Workspace["input"]);phase.SelectedItem=Text(Workspace["phase"]);foreach(Choice choice in workGoal.Items)if(choice.Id==Text(Value(Workspace,"goal")))workGoal.SelectedItem=choice;string savedGrip=Text(Value(Map(Value(Workspace,"view")),"grip_mode"));if(savedGrip=="hold"||savedGrip=="latch")gripMode.SelectedItem=savedGrip;
   if(input!=null)input.RefreshContext();
   string bankPurpose=Text(Value(Bank,"purpose")).Split('/')[0].Trim();bankLabel.Text=Workspace["bank"]+" · "+bankPurpose+KeyHint("bank");tips.SetToolTip(bankLabel,Bank==null?"":"Keyboard: "+(Value(Bank,"orbit")==null?"Shared set":OrbitName(Number(Bank["orbit"])))+"\nWorking orbit: "+OrbitName(Number(Workspace["orbit"]))+"\n"+Text(Bank["requirements"]));
   if(selectedEffect!=null&&selectedEffectKey!=SelectedMacroEffectKey())selectedEffect=null;
   NativeDiagnostics.Write("Experiment draw: exact hub objects");var effect=selectedEffectScope=="body"?selectedEffect:(Text(Value(review,"status"))=="Stale"?null:Map(Value(review,"effect")));if(timing!=null)timing.HubStart=Stopwatch.GetTimestamp();try{hub.UpdateContext(work,effect,phaseInspection,phaseInspectionReason,selectedEffectScope,requestedPreview);}finally{if(timing!=null)timing.HubDone=Stopwatch.GetTimestamp();}
   var chosen=Items(work["library"]).Select(Map).FirstOrDefault(m=>Text(m["id"])==selectedMacro);
   string chosenName=chosen==null?"No selected macro":"Selected: "+MacroName(chosen);
   string scope=selectedEffectScope=="complete"?"Full operation":"Macro body";
   string reasons=selectedEffectScope!="body"||selectedEffect==null?"":selectedApplicability==null?"Applicability: recheck":String.Join("; ",Items(Value(selectedApplicability,"reasons")).Select(Text).ToArray());
   effectText.Text=effect==null?"Effect not checked":scope+" · "+effect["primitives"]+" turns\n"+effect["pieces"]+" pieces · "+effect["slots"]+" labels · "+effect["cycle_count"]+" cycles\n"+effect["fixed_orientation_count"]+" fixed-position frame changes";
   if(reasons.Length>0)effectText.Text+="\n"+(selectedApplicability==null?"Applicability: recheck":Items(Value(selectedApplicability,"reasons")).Length+" applicability issues · Details");
   string effectDetails=(selectedEffectScope=="complete"?"Prepare + Macro + Cleanup":chosenName+(chosen==null?"":"\nCanonical macro: "+selectedMacro))+"\n"+(effect==null?effectText.Text:EffectCaption(effect)+"\n"+scope+"\n"+reasons);tips.SetToolTip(effectText,effectDetails);effectText.AccessibleDescription=effectDetails;effectText.AccessibleName=scope+" exact effect; open details";
   foreach(var entry in commandButtons.Where(e=>!e.Key.IsDisposed)){string id=entry.Value;entry.Key.Text=Text(entry.Key.Tag)+KeyHint(id);tips.SetToolTip(entry.Key,hints[id]+KeyHint(id));}
   foreach(string p in phaseButtons.Keys){bool active=p==Text(Workspace["phase"]);var b=phaseButtons[p];b.Text=(active?"▶ ":"")+Char.ToUpperInvariant(p[0])+p.Substring(1)+" · "+Items(Map(Workspace["draft"])[p]).Length+KeyHint("phase-"+p);b.FlatStyle=active?FlatStyle.Flat:FlatStyle.System;b.BackColor=active?Color.FromArgb(220,235,238):SystemColors.Control;}
   foreach(var entry in commandButtons.Where(e=>!e.Key.IsDisposed&&e.Value=="macro-insert"))entry.Key.Text="Append to "+Text(phase)+KeyHint("macro-insert");
   var selected=Selection;pieceText.Text=selected==null?"Inspect a piece or position. Selection does not change Current.":"Actual · "+PieceCaption(selected)+(mode=="g2"&&inspectedBoundary!="actual"?"\nHub: "+BoundaryName(inspectedBoundary)+". Forecast geometry is not shown here.":"")+"\nHome C"+String.Join(", C",Items(selected["home_cells"]).Select(Text).ToArray())+"\nHosting C"+String.Join(", C",Items(selected["current_cells"]).Select(Text).ToArray())+"\n"+Items(selected["cap_cells"]).Length+" affecting caps; inspect exact mappings in Capture.";
   tips.SetToolTip(pieceText,pieceText.Text);
   local.OrbitCaption=global.OrbitCaption=OrbitName(Number(Map(snapshot.State["prefs"])["orbit"]));
   local.UpdateStatus(NativeCellStatus.FromState(snapshot.State));global.UpdateStatus(NativeCellStatus.FromState(snapshot.State));
   var localCenter=Value(Map(Value(Workspace,"view")),"local_center");if(localCenter!=null)local.SetCenter(Number(localCenter));if(localCenter!=null)local.CenterCaption=CellName(Number(localCenter));
   string linkedKey=selected==null?null:PieceCaption(selected);
   if(linkedKey!=linkedPieceContext){linkedPieceContext=linkedKey;if(selected!=null&&Items(selected["current_cells"]).Length>0){int c=Number(Items(selected["current_cells"])[0]);local.SetSelectedColor(c);global.SetSelectedColor(c);}}
   NativeDiagnostics.Write("Experiment draw: macro catalogue and keyboard");DrawMacros();DrawKeyboard();DrawCandidateCaption();DrawOperationState();DrawSessionState();FitWorkspace();UpdateWindowReadouts();NativeDiagnostics.Write("Experiment draw: complete");
  }finally{updating=false;if(timing!=null)timing.DrawDone=Stopwatch.GetTimestamp();}
 }
 static string PieceCaption(Dictionary<string,object> piece){if(piece==null)return "unassigned";var names=Map(Value(piece,"names"));return names==null?"I"+piece["piece"]+" at P"+piece["position"]:Text(Value(names,"identity_name"))+" · "+Text(Value(names,"current_name"));}
 string SelectedMacroEffectKey(){var record=Items(Value(work,"library")).Select(Map).FirstOrDefault(m=>Text(Value(m,"id"))==selectedMacro);return record==null?null:api.Json(LocalApi.D("id",record["id"],"version",record["version"],"recipe",record["recipe"]));}
 static string EffectCaption(Dictionary<string,object> effect){return Text(effect["category"])+" · "+effect["primitives"]+" primitives\n"+effect["pieces"]+" pieces / "+effect["slots"]+" labels · "+effect["cycle_count"]+" cycles · "+effect["fixed_orientation_count"]+" fixed-position frame changes";}
 void DrawMacros(){if(work==null)return;bool before=updating;updating=true;try{if(macroUsePicker.Items.Count<=4&&structure!=null)PopulateMacroUses(macroUsePicker);string term=search.Text.Trim();var entries=CandidateOrder(Items(work["library"]).Select(Map).Where(m=>NativeMacroFacets.MatchesUse(m,macroUseFilter,Number(Workspace["orbit"]))&&MacroMatchesFacets(m)&&(MacroName(m)+" "+OrbitName(Number(m["orbit"]))+" "+MacroUseSummary(m)+" "+m["id"]+" "+m["note"]).IndexOf(term,StringComparison.OrdinalIgnoreCase)>=0)).Select(m=>new Choice(Text(m["id"]),MacroName(m))).ToArray();bool same=macros.Items.Count==entries.Length&&macros.Items.Cast<Choice>().Select(c=>c.Id+"|"+c.Label).SequenceEqual(entries.Select(c=>c.Id+"|"+c.Label));if(!same){macros.BeginUpdate();try{macros.Items.Clear();macros.Items.AddRange(entries);}finally{macros.EndUpdate();}}int selectedIndex=-1;for(int i=0;i<macros.Items.Count;i++)if(((Choice)macros.Items[i]).Id==selectedMacro)selectedIndex=i;macros.SelectedIndex=selectedIndex;macros.Invalidate();if(mode=="g2")DrawWorkspace();}finally{updating=before;}}
 ExperimentInputState InputState(){
  var result=new ExperimentInputState{Enabled=connected&&stopFailure==null,Busy=OperationInputBlocked,Modal=modal,BankId=Text(Value(Workspace,"bank")),GripMode=Text(gripMode),Destination=Text(destination),Phase=Text(phase),FrameKey=api.Json(Value(Bank,"frames"))};
  var saved=Map(Value(Workspace,"keybinds"));var bankOverrides=Map(Value(Map(Value(saved,"banks")),result.BankId));
  var codes=Items(Value(bankOverrides,"grips"));if(codes.Length==0)codes=Items(Value(saved,"grips"));if(codes.Length==0)codes=gripCodes.Cast<object>().ToArray();
  bool turns=!Object.Equals(Value(Bank,"turns_enabled"),false);var slots=Items(Value(Bank,"slots"));if(turns)for(int i=0;i<Math.Min(codes.Length,slots.Length);i++)result.GripKeys[Text(codes[i])]=Number(slots[i]);
  result.TwistKeys=new Dictionary<string,string>(defaultTwists);result.CommandKeys=new Dictionary<string,string>(defaultKeys);
  result.TwistKeys["KeyK"]="T1-";result.TwistKeys["KeyL"]="T2-";result.TwistKeys["Semicolon"]="T3-";result.TwistKeys["Quote"]="T4-";
  if(result.BankId!="Functions"&&!Object.Equals(Value(Bank,"legacy"),true)){
   var functions=Items(Value(work,"banks")).Select(Map).FirstOrDefault(b=>Text(Value(b,"id"))=="Functions");var utilities=new HashSet<string>(Items(Value(functions,"functions_only")).Select(Text));
   foreach(string key in result.CommandKeys.Where(pair=>utilities.Contains(pair.Value)||pair.Key=="F6"||pair.Key=="F7").Select(pair=>pair.Key).ToArray())result.CommandKeys.Remove(key);
  }
  MergeKeys(result.CommandKeys,Map(Value(Bank,"commands")));MergeKeys(result.TwistKeys,Map(Value(saved,"twists")));MergeKeys(result.CommandKeys,Map(Value(saved,"commands")));MergeKeys(result.TwistKeys,Map(Value(bankOverrides,"twists")));MergeKeys(result.CommandKeys,Map(Value(bankOverrides,"commands")));if(!turns)result.TwistKeys.Clear();
  // This new convenience default must not take over a saved physical binding.
  var sharedCommands=Map(Value(saved,"commands"));var localCommands=Map(Value(bankOverrides,"commands"));
  PreserveConvenienceBinding(result,"Backquote",sharedCommands,localCommands);PreserveConvenienceBinding(result,"Backslash",sharedCommands,localCommands);
  return result;
 }
 internal static void PreserveConvenienceBinding(ExperimentInputState state,string code,Dictionary<string,object> shared,Dictionary<string,object> local){
  if((state.GripKeys.ContainsKey(code)||state.TwistKeys.ContainsKey(code))&&(shared==null||!shared.ContainsKey(code))&&(local==null||!local.ContainsKey(code)))state.CommandKeys.Remove(code);
 }
 internal static string FunctionsDestination(string current,string previous,IEnumerable<string> available){
  string target=current=="Functions"?previous:"Functions";
  if(String.IsNullOrEmpty(target)||target==current||!available.Contains(target))throw new InvalidOperationException("The previous keyboard set is unavailable. Use Sets to choose it explicitly; no set was substituted.");return target;
 }
 string FunctionsShortcutNotice(){return InputState().CommandKeys.Any(pair=>pair.Value=="functions-toggle"&&!pair.Key.Contains("+"))?"":"Functions has no single-key route in this set. Commands → Functions → Change key; existing bindings are retained.";}
 internal static void MergeKeys(Dictionary<string,string> target,Dictionary<string,object> additions){
  if(additions==null)return;var normalized=new Dictionary<string,object>();
  foreach(var pair in additions){string key=pair.Key.StartsWith("Ctrl+",StringComparison.Ordinal)?"Control+"+pair.Key.Substring(5):pair.Key;object prior;if(normalized.TryGetValue(key,out prior)&&!Object.Equals(prior,pair.Value))throw new ArgumentException("Two bindings describe "+key+". Keep one binding for that key.");normalized[key]=pair.Value;}
  foreach(var pair in normalized){if(pair.Value==null)target.Remove(pair.Key);else target[pair.Key]=Text(pair.Value);}
 }
 object CheckedBindings(string json){var saved=api.Parse(json);var root=Map(saved);ValidateBindings(root);Action<Dictionary<string,object>> check=group=>{var map=Map(Value(group,"commands"));if(map!=null)foreach(var binding in map)if(binding.Value!=null&&!commands.ContainsKey(Text(binding.Value)))throw new ArgumentException("Unknown command '"+Text(binding.Value)+"'. Choose its ID from Commands.");};check(root);var banks=Map(Value(root,"banks"));if(banks!=null)foreach(var bank in banks)check(Map(bank.Value));return saved;}
 internal static void ValidateBindings(Dictionary<string,object> saved){
  if(saved==null)throw new ArgumentException("Keyboard bindings must be a JSON object.");
  Action<Dictionary<string,object>> check=group=>{foreach(string section in new[]{"commands","twists"}){object value=Value(group,section);if(value==null)continue;var map=Map(value);if(map==null)throw new ArgumentException(section+" bindings must be a JSON object.");if(map.Values.Any(v=>v!=null&&!(v is string)))throw new ArgumentException(section+" bindings must name an action or use null to unbind.");MergeKeys(new Dictionary<string,string>(),map);}};
  check(saved);object banks=Value(saved,"banks");if(banks!=null){var groups=Map(banks);if(groups==null)throw new ArgumentException("Bank overrides must be a JSON object.");foreach(var group in groups){var map=Map(group.Value);if(map==null)throw new ArgumentException("Bank "+group.Key+" must contain an object.");check(map);}}
 }
 Dictionary<string,object> keyHintWork;
 Dictionary<string,string> keyHintCommands;
 string KeyHint(string command){long started=sendTimingEnabled?Stopwatch.GetTimestamp():0;try{
  // Command bindings belong to the immutable adopted reply. Live busy/focus/grip state is never cached.
  if(input!=null&&(keyHintCommands==null||!Object.ReferenceEquals(keyHintWork,work))){keyHintCommands=InputState().CommandKeys;keyHintWork=work;}
  var map=input==null?defaultKeys:keyHintCommands;var key=map.Where(p=>p.Value==command).Select(p=>p.Key).OrderBy(k=>k.Length).FirstOrDefault();return key==null?"":"  "+ShortCode(key);
 }finally{if(sendTimingEnabled){sendTimingKeyHintCalls++;sendTimingKeyHintTicks+=Stopwatch.GetTimestamp()-started;}}}
 string CommandLabel(string id){return hints[id]+KeyHint(id);}
 void ShowCatalogue(bool visible){if(mode=="g2"){if(visible)OpenWorkWindow("macro");else HideWorkWindow("macro");FitWorkspace();return;}catalogue.Visible=visible;FitWorkspace();if(visible){form.Activate();search.Focus();}else FocusWorkspace();}
 void ShowKeyboard(){
  if(keyboardWindow!=null&&!keyboardWindow.IsDisposed&&keyboardWindow.Visible&&keyboardWindow.WindowState!=FormWindowState.Minimized){HideWorkWindow("keyboard");return;}OpenFloatingKeyboard(true);
 }
 void SelectBankMacro(int index){var entries=Items(Value(Bank,"macros"));if(index<0||index>=entries.Length)throw new InvalidOperationException("This bank has no macro in that slot.");string id=Text(entries[index]);if(!Items(work["library"]).Select(Map).Any(m=>Text(m["id"])==id))throw new InvalidOperationException("The recorded macro is missing: "+id);selectedMacro=id;selectedEffect=null;if(mode=="g2")OpenWorkWindow("macro",false);else catalogue.Visible=true;FitWorkspace();bool chosenSearch=search.Focused;EventHandler entered=delegate{chosenSearch=true;};search.Enter+=entered;Send(LocalApi.D("action","macro-effect","id",id,"select",true)).ContinueWith(t=>OnUi(()=>{search.Enter-=entered;if(t.Status==TaskStatus.RanToCompletion&&t.Result&&!chosenSearch&&(Form.ActiveForm==form||Form.ActiveForm==keyboardWindow))FocusWorkspace();}));}
 void RefreshCommandAvailability(){
  RefreshCommandAvailabilityTimed(null);
 }
 void RefreshCommandAvailabilityTimed(SendTiming timing){
  bool ready=IsReady;hub.SetInteractionEnabled(ready);if(timing!=null)timing.AvailabilityHubDone=Stopwatch.GetTimestamp();macros.Enabled=ready;workGoal.Enabled=ready;bridge.SetBusy(!ready);if(timing!=null)timing.AvailabilityBridgeDone=Stopwatch.GetTimestamp();input.RefreshFocusFeedback();if(timing!=null)timing.AvailabilityInputDone=Stopwatch.GetTimestamp();
  if(ready&&pendingDefaultViews){ResetPuzzleView();pendingDefaultViews=false;}
  DrawRecommendation();DrawKnownEndgame();
  if(stopFailure!=null)Say(stopFailure,true);else if(stopPending)Say("Waiting for stop acknowledgement; operation input remains paused.");
  if(ready)OnUi(()=>ShowSessionCompletion());
 }
 void StopAnalysis(){
  if(closing||stopPending)return;if(stopFailure!=null){Say(stopFailure,true);return;}if(!busy){Say("No active check to stop.");return;}
  stopPending=true;var completion=new TaskCompletionSource<bool>();stopCompletion=completion;RefreshCommandAvailability();
  Task.Factory.StartNew(()=>api.Post("stop-job",LocalApi.D())).ContinueWith(t=>OnUi(delegate{
   bool acknowledged=!t.IsFaulted&&!t.IsCanceled&&Object.Equals(Value(t.Result,"cancel_requested"),true);
   stopPending=false;
   if(!acknowledged){string reason=t.IsFaulted?t.Exception.GetBaseException().Message:t.IsCanceled?"The stop request was interrupted.":"The server did not acknowledge this stop request.";stopFailure="Stop outcome is uncertain: "+reason+" Operation input is disabled. Relaunch this isolated session before continuing.";NativeDiagnostics.Write(stopFailure);}
   RefreshCommandAvailability();if(acknowledged)Say(busy?"Stop acknowledged; waiting for the active operation result.":"Stop acknowledged; the operation result is available.");completion.TrySetResult(acknowledged);
  },()=>completion.TrySetCanceled()));
 }
 internal static int[] ReviewConflictPositions(Dictionary<string,object> review){
  if(review==null||Text(Value(review,"status"))=="Stale")throw new InvalidOperationException("Review the current operation before locating its conflicts.");
  var prefix=Map(Value(review,"prefix"));var intermediate=Text(Value(prefix,"status"))=="Violation"?Items(Value(prefix,"conflicting_positions")):new object[0];
  return Items(Value(review,"block_conflicts")).Concat(intermediate).Select(Number).Distinct().OrderBy(p=>p).ToArray();
 }
 void ShowReview(){
  var review=Map(Value(work,"review"));if(review==null)throw new InvalidOperationException("Review Prepare, Macro and Cleanup first.");
  if(Text(review["status"])=="Stale"){ShowReviewFindings(new List<string>{"Out of date — check the complete operation again.","Historical protection and score conclusions are withdrawn."});return;}
  bool stale=Text(review["status"])=="Stale";var lines=new List<string>{stale?"OUT OF DATE — review again before using these results.":"Operation check: "+Text(review["status"]),"","At the end: "+(Items(review["conflicts"]).Length==0&&Items(review["block_conflicts"]).Length==0?"protected state is preserved.":"protected state would change.")};
  foreach(var item in Items(review["conflicts"]).Select(Map))lines.Add("• "+OrbitName(Number(item["orbit"]))+": "+item["pieces"]+" pieces / "+item["stickers"]+" labels affected, including hidden pieces.");
  foreach(var item in Items(review["block_conflicts"]))lines.Add("• Protected position P"+item+" would change. Use Locate conflict to inspect it.");
  var prefix=Map(review["prefix"]);string p=Text(prefix["status"]);lines.Add("During the operation: "+(p=="Unchecked"?"not checked.":p=="Violation"?"protected state first moves at turn "+prefix["first_violation"]+" in the expanded operation.":"protected state remains unchanged at every checked step."));
  if(p=="Violation"){var positions=Items(Value(prefix,"conflicting_positions"));if(positions.Length>0){lines.Add("• Conflicts at that turn: "+String.Join(", ",positions.Select(position=>"P"+position))+". Use Locate conflict to inspect a position.");if(Object.Equals(Value(prefix,"conflicting_positions_truncated"),true))lines.Add("Showing "+positions.Length+" of "+Value(prefix,"conflicting_positions_total")+" conflicting positions at that turn.");}}
  lines.Add("");string goal=Text(Value(review,"goal"));var goalResult=Map(Value(review,"goal_result"));
  if(goalResult!=null){lines.Add(SheetGoal(goal)+" · "+Text(Value(goalResult,"status")));lines.Add(Text(Value(goalResult,"reason")));}
  else {bool met=Object.Equals(Value(review,"goal_met")??Value(review,"target_met"),true);lines.Add(SheetGoal(goal)+" · "+(goal=="prepare"?"No immediate target claim":met?"Predicted goal satisfied":"Goal not yet met"));}
  lines.AddRange(Items(review["reasons"]).Select(Text));
  ShowReviewFindings(lines);
 }
 void DrawKeyboard(){RenderKeyboard();}
 internal static string ShortCode(string code){if(code.Contains("+"))return String.Join("+",code.Split('+').Select(ShortCode).ToArray());switch(code){case "Control":case "Ctrl":return "Ctrl";case "ArrowLeft":return "←";case "ArrowRight":return "→";case "ArrowUp":return "↑";case "ArrowDown":return "↓";case "NumpadEnter":return "Num ↵";case "NumpadDivide":return "Num /";case "NumpadMultiply":return "Num ×";case "NumpadSubtract":return "Num −";case "NumpadAdd":return "Num +";case "NumpadDecimal":return "Num .";case "PageUp":return "PgUp";case "PageDown":return "PgDn";case "Escape":return "Esc";case "Backquote":return "`";case "Backslash":return "\\";case "Minus":return "−";case "Equal":return "=";case "BracketLeft":return "[";case "BracketRight":return "]";case "Semicolon":return ";";case "Quote":return "'";case "Comma":return ",";case "Period":return ".";case "Slash":return "/";case "Backspace":return "Bksp";default:return code.StartsWith("Numpad",StringComparison.Ordinal)?"Num "+code.Substring(6):code.StartsWith("Digit",StringComparison.Ordinal)?code.Substring(5):code.StartsWith("Key",StringComparison.Ordinal)?code.Substring(3):code;}}
 void DrawGripFeedback(){if(input==null)return;local.SetGripCap(input.ActiveGripCell);hub.SetGripCap(input.ActiveGripCell);RenderGripFeedback();RefreshSolveGrip();}
 void SelectCell(int cell){local.SetSelectedColor(cell);global.SetSelectedColor(cell);pieceText.Text="C"+cell+" selected in structural view. Context and locked Next are unchanged. Use the view context menu to locate the native camera.";}
 void HubAction(string action,string kind,int id){
  try{
   if(hub.SelectionIsForecast&&kind=="identity"&&action!="set-current"&&action!="pin-next")throw new InvalidOperationException("Select a fixed position or actual token before changing a position-based role or protection. Forecast identity actions only track that identity.");
   if(busy||Selection==null||kind!="identity"&&kind!="position"||Number(Selection[kind=="identity"?"piece":"position"])!=id)
    throw new InvalidOperationException("This object has not completed exact inspection. Inspect it again before assigning a role or bookmark.");
   int identity=kind=="identity"?id:SelectedIdentity();int position=kind=="position"?id:SelectedPosition();
   if(action=="set-current")Send(LocalApi.D("action","focus","identity",identity));
   else if(action=="pin-next")PinNext(identity);
   else if(action=="protect-position")Send(LocalApi.D("action","block-protect","position",position));
   else if(action=="assign-target"){if(work["current"]==null)throw new InvalidOperationException("Choose Current before assigning its destination.");Send(LocalApi.D("action","focus","identity",Map(work["current"])["piece"],"target",position));}
   else{var roles=(object[])Items(Workspace["roles"]).Clone();roles[action=="assign-a"?0:1]=position;Send(LocalApi.D("action","roles","positions",roles));}
  }catch(Exception e){Say(e.Message,true);}
 }
 void PinNext(int id){if(work["next"]==null){Send(LocalApi.D("action","next-pin","identity",id));return;}Edit("Replace locked Next",new[]{"Piece identity · I number or copied Piece address"},new[]{"I"+id},values=>Send(LocalApi.D("action","next-pin","identity",ParseObjectInput(values[0],'I'),"replace",true)),"Replace explicitly");}
 static int ParseId(string text,char prefix,int limit){string value=text.Trim();if(value.Length>0&&value[0]==prefix)value=value.Substring(1);int number;if(value.Length==0||value.Any(c=>c<'0'||c>'9')||!Int32.TryParse(value,out number)||number<0||number>=limit)throw new ArgumentException("Expected "+prefix+"0.."+prefix+(limit-1)+". The input was preserved; no object was selected.");return number;}
 static object ParseObjectInput(string text,char prefix){string value=text.Trim();if(value.StartsWith("Magic600.",StringComparison.Ordinal))return value;try{return ParseId(value,prefix,177120);}catch(ArgumentException){throw new ArgumentException("Expected "+prefix+"0.."+prefix+"177119 or a complete copied "+(prefix=='I'?"Piece":"Position")+" address. The input was preserved; no object was selected.");}}
 void CopySelectionAddress(){var selected=Selection;if(selected==null)throw new InvalidOperationException("Inspect a piece or fixed position first.");bool position=Value(Workspace,"inspected_position")!=null;var names=Map(Value(selected,"names"));string copied=position?Text(Value(Map(Value(names,"current")),"copy_text")):Text(Value(names,"identity_copy_text"));if(copied.Length==0||position&&Number(selected["position"])!=Number(Workspace["inspected_position"]))throw new InvalidOperationException("The inspected address is unavailable. Inspect the object again before copying.");Clipboard.SetText(copied);Say(position?"Copied Position address · canonical P"+selected["position"]:"Copied Piece address · canonical I"+selected["piece"]);}
 void Edit(string title,string[] names,string[] values,Func<string[],Task> apply,string button="Apply explicitly"){
  var dialog=new Form{Text=title,Font=form.Font,StartPosition=FormStartPosition.CenterParent,ClientSize=new Size(650,390),MinimumSize=new Size(460,300),ShowInTaskbar=false,Owner=form};
  var layout=new TableLayoutPanel{Dock=DockStyle.Fill,ColumnCount=1,RowCount=names.Length*2+2,Padding=new Padding(12)};var fields=new List<TextBox>();
  for(int i=0;i<names.Length;i++){layout.Controls.Add(new Label{Text=names[i],Dock=DockStyle.Fill,AutoSize=true},0,i*2);bool readOnly=names[i].StartsWith("Read-only",StringComparison.Ordinal);var edit=new TextBox{Text=values[i],Dock=DockStyle.Fill,ReadOnly=readOnly,Multiline=readOnly||names[i].IndexOf("JSON",StringComparison.Ordinal)>=0,AcceptsReturn=true,ScrollBars=ScrollBars.Vertical,AccessibleName=names[i]};fields.Add(edit);layout.RowStyles.Add(new RowStyle(SizeType.AutoSize));layout.RowStyles.Add(new RowStyle(edit.Multiline?SizeType.Percent:SizeType.Absolute,edit.Multiline?100:34));layout.Controls.Add(edit,0,i*2+1);}
  var error=new Label{AutoSize=true,ForeColor=Color.FromArgb(172,58,45),Dock=DockStyle.Fill};layout.Controls.Add(error,0,names.Length*2);
  var actions=Row();var ok=new Button{Text=button,AutoSize=true,MinimumSize=new Size(130,32)};var cancel=new Button{Text="Cancel",DialogResult=DialogResult.Cancel,AutoSize=true,MinimumSize=new Size(90,32)};actions.Controls.Add(ok);actions.Controls.Add(cancel);layout.Controls.Add(actions,0,names.Length*2+1);dialog.Controls.Add(layout);dialog.CancelButton=cancel;
  ok.Click+=async delegate{ok.Enabled=false;cancel.Enabled=false;foreach(var field in fields)field.Enabled=false;try{var task=apply(fields.Select(f=>f.Text).ToArray());if(task!=null)await task;var outcome=task as Task<bool>;if(outcome!=null&&!outcome.Result){error.Text=feedback.Text;return;}dialog.Close();}catch(Exception e){error.Text=e.Message;}finally{if(!ok.IsDisposed){ok.Enabled=true;cancel.Enabled=true;foreach(var field in fields)field.Enabled=true;}}};
  ShowOwned(dialog);dialog.Dispose();
 }
 void FocusWorkspace(){if(closing)return;form.Activate();if(mode=="g2"&&hub.FocusWork())return;bridge.Viewport.Focus();}
 bool pendingDefaultViews;
 void ResetPuzzleView(){bridge.ResetView();local.ResetView();global.ResetView();}
 void ShowOwned(Form dialog){StyleTool(dialog);var owner=Form.ActiveForm;if(owner==null||owner!=form&&!windows.Contains(owner))owner=form;var prior=owner.ActiveControl;modal=true;windows.Add(dialog);input.RegisterWindow(dialog);input.Reset("Editor owns input.");EventHandler focusChanged=delegate{UpdateWindowReadouts();};dialog.Activated+=focusChanged;try{dialog.Owner=owner;PlaceOwnedDialog(dialog,owner);dialog.ShowDialog(owner);}finally{dialog.Activated-=focusChanged;modal=false;windows.Remove(dialog);input.Reset("Editor closed; release held keys.");if(!owner.IsDisposed&&owner.Visible)owner.Activate();if(prior==null||prior.IsDisposed||!prior.Focus())FocusWorkspace();input.RefreshFocusFeedback();UpdateWindowReadouts();OnUi(()=>ShowSessionCompletion());}}
 void Choose(string title,IEnumerable<Choice> choices,Action<string> choose){
  var all=choices.ToArray();var dialog=new Form{Text=title,Font=form.Font,ClientSize=new Size(660,460),MinimumSize=new Size(460,300),StartPosition=FormStartPosition.CenterParent,Owner=form};
  var query=new TextBox{Dock=DockStyle.Top,AccessibleName=title+" search or exact ID"};var list=new ListBox{Dock=DockStyle.Fill,IntegralHeight=false,AccessibleName=title+" entries"};var apply=new Button{Text="Activate selected entry",Dock=DockStyle.Bottom,Height=36};
  Action fill=delegate{list.Items.Clear();foreach(var c in all)if((c.Id+" "+c.Label).IndexOf(query.Text,StringComparison.OrdinalIgnoreCase)>=0)list.Items.Add(c);if(list.Items.Count==1)list.SelectedIndex=0;else for(int i=0;i<list.Items.Count;i++)if(((Choice)list.Items[i]).Id==query.Text.Trim())list.SelectedIndex=i;};query.TextChanged+=delegate{fill();};fill();
  string accepted=null;Action accept=delegate{var selected=all.FirstOrDefault(c=>c.Id==query.Text.Trim())??list.SelectedItem as Choice;if(selected==null)return;accepted=selected.Id;dialog.Close();};apply.Click+=delegate{accept();};list.DoubleClick+=delegate{accept();};dialog.AcceptButton=apply;dialog.Controls.Add(list);dialog.Controls.Add(query);dialog.Controls.Add(apply);ShowOwned(dialog);dialog.Dispose();if(accepted!=null)choose(accepted);
 }
 void EditPhase(string name){EditSteps(name);}
 void RegisterCommands(){
  RegisterSessionCommands();RegisterKeymapFileCommands();RegisterDisplayCommands();
  RegisterPhaseInspection();RegisterWindowCommands();RegisterHelpCommands();
  defaultKeys["Control+KeyN"]="operation-new";defaultKeys["Control+Alt+KeyR"]="operation-reuse";
  Register("operation-new","Start a new operation; empty all three phases",()=>Send(LocalApi.D("action","operation-new")));
  Register("operation-reuse","Reuse these exact steps; requires fresh check",()=>Send(LocalApi.D("action","operation-reuse")));
  foreach(string id in new[]{"set-current","assign-a","assign-b","assign-target"}){string action=id;Register(id,id=="set-current"?"Make inspected piece Current":id=="assign-target"?"Use inspected position as Target":"Use inspected position as "+id.Substring(id.Length-1).ToUpperInvariant(),()=>HubAction(action,hub.SelectedKind,hub.SelectedId));}
  Register("index","Command index",ShowCommandPalette);
  Register("solve-actions","Actions for the selected work object",()=>{if(!hub.FocusWork())ShowCommandPalette();});
  foreach(string p in new[]{"prepare","macro","cleanup"}){string phaseName=p;Register("phase-"+p,"Select "+p+" input phase",()=>ChooseInputPhase(phaseName));Register("edit-"+p,"Edit "+p+" recipe",()=>EditPhase(phaseName));}
  Register("input-draft","Send turns to the draft",()=>Send(LocalApi.D("action","settings","input","draft")));
  Register("input-live","Apply explicitly entered turns to the puzzle",()=>Send(LocalApi.D("action","settings","input","live")));
  Register("prefix-on","Protect every intermediate step",()=>Send(LocalApi.D("action","prefix","strict",true)));Register("prefix-off","Protect the end result only",()=>Send(LocalApi.D("action","prefix","strict",false)));
  Register("bank","Switch keyboard bank",ShowBankPicker);
  defaultKeys["Backslash"]="functions-toggle";
  Register("functions-toggle","Open Functions, or return to the previous keyboard set",()=>{string target=FunctionsDestination(Text(Workspace["bank"]),Text(Value(Workspace,"previous_bank")),Items(work["banks"]).Select(Map).Select(b=>Text(b["id"])));Send(LocalApi.D("action","bank","id",target));});
  Register("bank-Functions","Use Functions keyboard set",()=>Send(LocalApi.D("action","bank","id","Functions")));
  Register("orbit","Choose working orbit",()=>Choose("Working orbit",Items(Value(structure,"orbit_profiles")).Select(Map).Select(p=>new Choice(Text(p["orbit"]),OrbitName(Number(p["orbit"])))),id=>Send(LocalApi.D("action","orbit","orbit",Int32.Parse(id)))));
  foreach(string suffix in new[]{"A","B","I","M","E"}){string s=suffix;Register("bank-"+s,"Switch to "+s+" in the working orbit",()=>Send(LocalApi.D("action","bank","id",Workspace["orbit"]+"-"+s)));}
  Register("bank-previous","Return to previous bank",()=>Send(LocalApi.D("action","bank","id",Workspace["previous_bank"])));
  Register("keyboard","Show or hide onscreen keyboard",ShowKeyboard);
  Register("keyboard-extra","Show or hide extra keyboard shortcuts",ToggleKeyboardExtras);
  Register("grip-hold","Use held Grip keys",()=>Send(LocalApi.D("action","settings","view",LocalApi.D("grip_mode","hold"))));
  Register("grip-latch","Use latched Grip keys",()=>Send(LocalApi.D("action","settings","view",LocalApi.D("grip_mode","latch"))));
  Register("key-edit","Edit a command key",()=>ShowBindingEditor("macro-insert"));
  Register("key-edit-advanced","Edit complete physical and command binding record",()=>Edit("Complete keyboard configuration",new[]{"Bindings JSON: grips, twists, commands and per-bank overrides"},new[]{api.Json(Workspace["keybinds"])},v=>Send(LocalApi.D("action","settings","keybinds",CheckedBindings(v[0])))));
  Register("focus","Choose Current and fixed destination",()=>Edit("Current identity and destination",new[]{"Piece identity · I number or copied Piece address","Fixed destination · P number or copied Position address"},new[]{Selection==null?"":"I"+Selection["piece"],Selection==null?"":"P"+Selection["piece"]},v=>Send(LocalApi.D("action","focus","identity",ParseObjectInput(v[0],'I'),"target",ParseObjectInput(v[1],'P')))));
  Register("next-menu","Locked Next bookmark actions",()=>Choose("Next is an identity bookmark, not mechanical protection",new[]{new Choice("next-pin","Pin / explicitly replace"),new Choice("next-locate","Locate without activating"),new Choice("next-activate","Activate explicitly"),new Choice("next-clear","Clear bookmark")},RunCommand));
  Register("next-pin","Pin inspected piece as Next",()=>PinNext(SelectedIdentity()));Register("next-clear","Clear Next bookmark",()=>Send(LocalApi.D("action","next-clear")));Register("next-activate","Explicitly activate Next",()=>Send(LocalApi.D("action","next-activate")));
  Register("next-locate","Locate Next without changing Current",()=>{var p=Map(work["next"]);if(p==null)throw new InvalidOperationException("Next is not assigned.");Send(LocalApi.D("action","inspect","identity",p["piece"]));bridge.LocateCell(Number(Items(p["current_cells"])[0]));});
  Register("block-add","Add inspected identity to Home block",()=>Send(LocalApi.D("action","block-add","identity",SelectedIdentity())));Register("block-remove","Remove inspected identity requirement",()=>Send(LocalApi.D("action","block-remove","identity",SelectedIdentity())));
  Register("block-capture","Add inspected current state as a block requirement",()=>Send(LocalApi.D("action","block-add","identity",SelectedIdentity(),"position",SelectedPosition(),"capture_current",true)));
  Register("block-capture-position","Require inspected occupant at this position; orientation may change",()=>Send(LocalApi.D("action","block-add","identity",SelectedIdentity(),"position",SelectedPosition(),"capture_current",true,"mode","position")));
  Register("block-protect-position","Preserve inspected occupant at this position; allow orientation changes",()=>Send(LocalApi.D("action","block-protect","position",SelectedPosition(),"mode","position")));
  Register("block-unprotect-position","Remove only position-preservation at inspected position",()=>Send(LocalApi.D("action","block-protect","position",SelectedPosition(),"enabled",false,"mode","position")));
  Register("block-unprotect-exact","Remove only exact-label protection at inspected position",()=>Send(LocalApi.D("action","block-protect","position",SelectedPosition(),"enabled",false,"mode","exact")));
  Register("block-protect","Protect exact labels at inspected position",()=>Send(LocalApi.D("action","block-protect","position",SelectedPosition())));Register("block-unprotect","Remove inspected position protection",()=>Send(LocalApi.D("action","block-protect","position",SelectedPosition(),"enabled",false)));
  Register("protection","Edit protected orbit boundary",EditOrbitProtection);
  Register("roles","Assign fixed A and B positions",()=>Edit("Buffer role assignment",new[]{"A position · P number or copied Position address","B position · P number or copied Position address"},new[]{"P"+Items(Workspace["roles"])[0],"P"+Items(Workspace["roles"])[1]},v=>Send(LocalApi.D("action","roles","positions",new[]{ParseObjectInput(v[0],'P'),ParseObjectInput(v[1],'P')}))));
  foreach(string g in new[]{"prepare","insert","place","orient","finish-buffer","block","endgame"}){string goal=g;Register("goal-"+g,"Work goal: "+SheetGoal(g),()=>Send(LocalApi.D("action","goal","goal",goal)));}
  Register("target-capture","Require Current's exact present orientation at its target",()=>Send(LocalApi.D("action","target-capture")));
  Register("target-home","Require Current's exact Home sticker arrangement at its Home target",()=>Send(LocalApi.D("action","target-home")));
  Register("block-reference","Apply an explicit reference to block requirements",()=>Edit("Block destination reference",new[]{"Signed legal generators JSON · changes desired requirements only; existing protection stays fixed"},new[]{api.Json(Workspace["reference"])},v=>Send(LocalApi.D("action","block-reference","word",api.Parse(v[0])))));
  Register("reference","Edit explicit reference word",()=>Edit("Explicit reference R; no setup search",new[]{"Signed legal generators JSON; [] is canonical"},new[]{api.Json(Workspace["reference"])},v=>Send(LocalApi.D("action","reference","word",api.Parse(v[0])))));
  Register("reference-transform","Save selected macro with explicit R",()=>ShowMacroVariant("reference"));
  Register("macro-compare","Compare two explicitly chosen macros",ShowMacroComparison);
  Register("macro-inverse","Save an inverse of the selected macro",()=>ShowMacroVariant("inverse"));
  Register("macro-geometry","Map selected macro between explicit cell frames",ShowGeometryVariant);
  Register("macro-candidates","Compare current use of up to twelve visible library macros",CheckVisibleCandidates);
  Register("endgame-choices","Show explicit retained endgame families in Solve",ToggleKnownEndgame);
  Register("endgame-x","Use inspected position as endgame auxiliary X",()=>EndgameUseInspected(false));
  Register("endgame-y","Use inspected position as endgame auxiliary Y",()=>EndgameUseInspected(true));
  Register("endgame-q","Choose X fixed-frame element",()=>FocusEndgameElement(false));
  Register("endgame-r","Choose Y fixed-frame element",()=>FocusEndgameElement(true));
  Register("endgame-check","Check the explicitly entered endgame family",CheckKnownEndgame);
  Register("endgame-save","Save the checked explicit endgame as a macro",SaveKnownEndgame);
  Register("endgame-select-saved","Select the saved endgame macro without inserting",SelectSavedEndgame);
  Register("endgame-details","Inspect the exact endgame family and collateral",ShowEndgameDetails);
  Register("residual-details","Inspect current residuals and exact completion checks",ShowResidualDetails);
  Register("journal-delta","Inspect the last committed solver step",ShowJournalDelta);
  Register("orbit-following","Choose a later unfinished orbit explicitly",ChooseFollowingOrbit);
  RegisterMacroLibraryCommands();
  Register("solve-macros","Open Solve macros",()=>OpenSolvePage("macros"));
  Register("solve-prepare","Open Solve preparation",()=>OpenSolvePage("prepare"));
  Register("solve-protection","Open Solve protection",()=>OpenSolvePage("protection"));
  Register("solve-locate-a","Inspect fixed Buffer A",()=>InspectSolveRole("a"));
  Register("solve-locate-b","Inspect fixed Buffer B",()=>InspectSolveRole("b"));
  Register("solve-locate-target","Inspect fixed Target",()=>InspectSolveRole("target"));
  Register("macro-search","Open Macro Base catalogue",()=>ShowCatalogue(true));
  Register("macro-close","Return to the full graphical workspace",()=>ShowCatalogue(false));
  Register("bank-macro-1","Select bank macro 1",()=>SelectBankMacro(0));Register("bank-macro-2","Select bank macro 2",()=>SelectBankMacro(1));
  Register("macro-insert","Insert selected fixed macro into active phase",()=>UseSelectedMacro(true));
  Register("macro-replace","Replace active phase with the selected fixed macro",()=>UseSelectedMacro(false));
  Register("macro-new","Input a concrete macro",()=>Edit("Save a finite user-selected macro",new[]{"Human name","Recipe JSON","Purpose / notes"},new[]{"","[{\"kind\":\"word\",\"moves\":[1]}]",""},v=>Send(LocalApi.D("action","save-macro","name",v[0],"recipe",api.Parse(v[1]),"note",v[2]))));
  Register("macro-details","Inspect selected macro effects and full record",ShowSelectedMacroDetails);
  Register("phase-input","Edit active operation phase",()=>EditPhase(Text(phase)));Register("cleanup-inverse","Use inverse of explicitly chosen Prepare",()=>Send(LocalApi.D("action","inverse-cleanup")));
  Register("review","Review full Prepare / Macro / Cleanup",()=>{ShowOperation(true);Send(LocalApi.D("action","review"));});
  Register("review-details","Read operation check results",ShowReview);
  for(int i=0;i<3;i++){int index=i;Register("review-reason-"+(i+1),"Inspect computed operation reason "+(i+1),delegate{InspectReviewReason(index);});}
  Register("effect-body","Show selected macro's fixed action",()=>{RequireMacro();selectedEffectScope="body";if(selectedEffect==null||selectedEffectKey!=SelectedMacroEffectKey())Send(LocalApi.D("action","macro-effect","id",selectedMacro,"select",true));else Draw();});
  Register("effect-complete","Show complete operation effect",()=>{selectedEffectScope="complete";Draw();});
  Register("review-locate","Locate a conflicting protected position",()=>{var positions=ReviewConflictPositions(Map(Value(work,"review")));if(positions.Length==0)throw new InvalidOperationException("No individual protected-position conflicts are listed. Orbit conflicts are listed in Check results.");Choose("Conflicting protected positions",positions.Select(p=>new Choice(Text(p),"Position P"+p)),id=>Send(LocalApi.D("action","inspect-position","position",Int32.Parse(id))));});
  Register("operation-focus","Focus preview and execution controls",()=>{ShowOperation(true);FocusOperationControls();});
  Register("operation-hide","Close operation region and return to scene",()=>{ShowOperation(false);FocusWorkspace();});
  Register("compare-preview","Compare committed and exact predicted objects  F6",()=>{if(OperationExecuted)throw new InvalidOperationException("Choose New operation or Reuse steps before comparing a new result.");if(Value(work,"predicted")==null)throw new InvalidOperationException("Review the current complete draft before comparing its predicted objects.");hub.Predicted=!hub.Predicted;});
  Register("preview","Stage the exact reviewed operation",()=>{var r=Map(work["review"]);if(r==null)throw new InvalidOperationException("Review the complete operation first.");Send(LocalApi.D("action","preview","review_id",r["id"]));});
  Register("commit","Execute the explicitly staged operation",()=>Send(LocalApi.D("action","commit")));Register("cancel-preview","Cancel preview; keep draft",()=>Send(LocalApi.D("action","cancel-preview")));
  Register("cancel-analysis","Cancel the running analysis",StopAnalysis);
  Register("undo","Undo committed operation",()=>Send(LocalApi.D("action","undo")));Register("redo","Redo committed operation",()=>Send(LocalApi.D("action","redo")));
  Register("filter","Compose Piece Filter",()=>ShowPieceFilter());
  Register("capture","Capture and rebind explicit caps",()=>Edit("Grip capture · old "+(Bank==null?"":String.Join(", ",Items(Bank["slots"]).Select(c=>"C"+c).ToArray()))+" · retained world cap frame",new[]{"Canonical caps JSON: 1..600, at most 20, no gaps"},new[]{api.Json(Bank["slots"])},v=>Send(LocalApi.D("action","capture","cells",api.Parse(v[0])))));
  Register("capture-piece","Capture inspected piece's explicitly listed affecting caps",()=>{if(Selection==null)throw new InvalidOperationException("Inspect a piece first.");Edit("Review affecting caps before capture",new[]{"Affecting caps JSON; choose at most 20"},new[]{api.Json(Selection["cap_cells"])},v=>Send(LocalApi.D("action","capture","cells",api.Parse(v[0]))));});
  Register("worksheet","Save or reuse fixed work sheet",()=>Choose("Work sheets require fresh review on every reuse",new[]{new Choice("worksheet-save","Save current fixed recipes"),new Choice("worksheet-use","Reuse a saved work sheet")},RunCommand));
  Register("worksheet-save","Save fixed recipes and reference",()=>Edit("Save work sheet",new[]{"Name"},new[]{""},v=>Send(LocalApi.D("action","template-save","name",v[0]))));
  Register("worksheet-use","Reuse saved fixed work sheet",()=>Choose("Choose work sheet",Map(Workspace["templates"]).Keys.Select(n=>new Choice(n,n)),ShowWorkSheet));
  Register("checkpoint","Save checkpoint",()=>Edit("Save checkpoint",new[]{"Name"},new[]{""},v=>Send(LocalApi.D("action","checkpoint","name",v[0]))));
  Register("restore","Restore saved checkpoint",()=>Choose("Restore checkpoint",Items(work["checkpoints"]).Select(Map).Select(c=>new Choice(Text(c["name"]),Text(c["name"]))),n=>Send(LocalApi.D("action","restore","name",n))));
  Register("views","Open linked geometric view",()=>Choose("Linked native views",new[]{new Choice("local","Local hosting structure  F7"),new Choice("global","Global cell relationships  F8"),new Choice("puzzle","Actual MPUlt puzzle"),new Choice("views-close","Return to full graphical workspace")},RunCommand));
  Register("views-close","Close auxiliary view",()=>{foreach(string id in new[]{"local","global"})HideWorkWindow(id);if(mode=="g2")viewSplit.Panel2Collapsed=true;FocusWorkspace();});
  Register("local","Open Local structure",()=>OpenWorkWindow("local"));Register("global","Open Global structure",()=>OpenWorkWindow("global"));Register("puzzle","Open actual native puzzle",()=>{viewSplit.Panel2Collapsed=false;views.SelectedIndex=0;form.Activate();bridge.Viewport.Focus();});
  Register("copy-selection","Copy inspected Piece or Position address",CopySelectionAddress);
  Register("copy-selection-canonical","Copy canonical piece ID",()=>{int identity=SelectedIdentity();Clipboard.SetText("I"+identity);Say("Copied canonical piece ID I"+identity);});
  Register("fixture-e1","Load explicit synthetic E1 witness into a fresh solved session",()=>Send(LocalApi.D("action","fixture","name","e1")));
  Register("reset","Reset puzzle with a recovery checkpoint",()=>ShowSessionReset("puzzle"));
 }
 void RequireMacro(){if(selectedMacro==null)throw new InvalidOperationException("Select an existing macro explicitly.");}
 public bool PreFilterMessage(ref Message message){
  // The original Form1 retains its own accelerators. Input routes above have
  // first refusal; never let an unhandled letter invoke an old solving action.
  if(!OwnedActiveWindow()||message.Msg!=0x100&&message.Msg!=0x104)return false;
  var target=Control.FromChildHandle(message.HWnd);if(target is BindingCapture)return false;for(Control c=target;c!=null;c=c.Parent)if(c is TextBoxBase||c is ComboBox||c is NumericUpDown)return false;
  var key=(Keys)message.WParam.ToInt32();var modifiers=Control.ModifierKeys;
  var focusedButton=target as Button;string focusedCommand;
  if(key==Keys.Return&&focusedButton!=null&&focusedButton.Parent==operationStrip&&commandButtons.TryGetValue(focusedButton,out focusedCommand)&&(focusedCommand=="review"||focusedCommand=="preview"||focusedCommand=="commit"||focusedCommand=="cancel-preview")){
   // A held Enter must not gain a new action when modifiers or focus change.
   if((message.LParam.ToInt64()&(1L<<30))!=0&&((modifiers&Keys.Control)!=0))return true;
   if(modifiers==(Keys.Control|Keys.Shift)){RunCommand("commit");return true;}if(modifiers==Keys.Control){RunCommand("preview");return true;}
  }
  if(key==Keys.F10&&modifiers==Keys.Shift)for(Control c=target;c!=null;c=c.Parent)if(c is ExperimentHub)return false;
  if(key==Keys.F4&&(modifiers&Keys.Alt)!=0){Form.ActiveForm.Close();return true;}
  if(key==Keys.Return||key==Keys.Space){var button=target as Button;if(button!=null&&modifiers==Keys.None){if(button.Enabled)button.PerformClick();return true;}if(key==Keys.Return)return true;}
  if(key==Keys.Tab||key==Keys.Space||key==Keys.Escape||key>=Keys.Left&&key<=Keys.Down)return false;
  return key>=Keys.A&&key<=Keys.Z||key>=Keys.D0&&key<=Keys.D9||key>=Keys.F1&&key<=Keys.F12;
 }
 public void Dispose(){closing=true;CloseWorkWindows();if(input!=null)input.Dispose();Application.RemoveMessageFilter(this);bridge.Dispose();tips.Dispose();}
}

internal static class ExperimentProgram {
 [STAThread] internal static int Main(string[] args){
  if(args.Length!=4)return 2;Program.UseEnglishUi();string exe=Path.GetFullPath(args[0]);Directory.SetCurrentDirectory(Path.GetDirectoryName(exe));
  AppDomain.CurrentDomain.AssemblyResolve+=delegate(object sender,ResolveEventArgs e){string path=Path.Combine(Path.GetDirectoryName(exe),new AssemblyName(e.Name).Name+".dll");return File.Exists(path)?Assembly.LoadFrom(path):null;};
  Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);ExperimentShell shell=null;
  Application.ThreadException+=delegate(object sender,System.Threading.ThreadExceptionEventArgs e){if(shell!=null&&shell.Recover(e.Exception))return;NativeDiagnostics.Write("Experimental native UI failure",e.Exception);MessageBox.Show(e.Exception.ToString(),"Magic 600 Cell native experiment");Application.Exit();};
  try{var assembly=Assembly.LoadFrom(exe);var form=(Form)Activator.CreateInstance(assembly.GetType("_3dedit.Form1",true));shell=new ExperimentShell(form,exe,args[1],args[2],args[3]);form.Text+=" · "+new DirectoryInfo(AppDomain.CurrentDomain.BaseDirectory).Name;Application.Run(form);return 0;}
  catch(Exception e){while(e is TargetInvocationException&&e.InnerException!=null)e=e.InnerException;NativeDiagnostics.Write("Experimental native startup failed",e);File.WriteAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"native-start-error.txt"),e.ToString());MessageBox.Show(e.ToString(),"Magic 600 Cell native startup");return 1;}
 }
}
