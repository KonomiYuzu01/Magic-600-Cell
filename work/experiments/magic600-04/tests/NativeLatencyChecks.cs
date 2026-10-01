// Test-only measurement of the real command/adoption/render boundaries.
// Injected application input is not physical key latency. Present return is not GPU completion.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class NativeLatencyChecks {
 [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
 const BindingFlags Flags=BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.Public;
 static readonly string[] Axes={"H1","H2","H3","T1","T2","T3","T4"};
 static T Field<T>(object owner,string name){return (T)owner.GetType().GetField(name,Flags).GetValue(owner);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static object Value(Dictionary<string,object> value,string key){object result;return value!=null&&value.TryGetValue(key,out result)?result:null;}
 static string Text(object value){return Convert.ToString(value);}
 static object[] Items(object value){return value==null?new object[0]:(object[])value;}
 static Dictionary<string,object> Work(ExperimentShell shell){return Map(shell.Work["workspace"]);}
 static string Hash(ExperimentShell shell){return Text(shell.Work["hash"]);}
 static long Tick(){return Stopwatch.GetTimestamp();}
 static double Milliseconds(long first,long last){return (last-first)*1000.0/Stopwatch.Frequency;}
 static void Require(bool value,string message){if(!value)throw new InvalidOperationException(message);}
 static void Invoke(ExperimentShell shell,string command){typeof(ExperimentShell).GetMethod("RunCommand",Flags).Invoke(shell,new object[]{command});}
 static object Bounds(Control c){var r=c.RectangleToScreen(c.ClientRectangle);return new{x=r.X,y=r.Y,width=r.Width,height=r.Height};}
 static void Foreground(Control control){var owner=control.FindForm();Require(owner!=null&&owner.Visible&&owner.WindowState!=FormWindowState.Minimized&&control.Visible&&control.IsHandleCreated&&control.Width>0&&control.Height>0,"Measured control is not a visible native surface");Require(GetForegroundWindow()==owner.Handle,"Measured native window does not own the actual Windows foreground");}
 sealed class Sample {
  internal string Kind,Action;internal int Number,Pair;internal bool Warmup;
  internal long Start,Dispatch,Adopt,Paint;internal NativeSnapshot PaintedSnapshot;internal string PaintBinding;
  internal bool Accepted,Measured,BusyAtEntry;internal Task TwistTask;internal List<Frame> Frames=new List<Frame>();
 }
 sealed class Frame {
  internal long RenderStart,RenderReturn,PresentStart,PresentReturn;
  internal NativeSnapshot Snapshot;internal byte[] Field;internal bool Foreground,Motion;internal int Visible,Drawn;
 }
 // Observe the end of an actual WM_PAINT, not Control.Paint (Local raises that before drawing).
 sealed class PaintReceipt:NativeWindow,IDisposable {
  readonly Action completed;
  internal PaintReceipt(Control control,Action completed){Require(control.IsHandleCreated,"Paint receipt requires a created control handle");this.completed=completed;AssignHandle(control.Handle);}
  protected override void WndProc(ref Message message){int kind=message.Msg;base.WndProc(ref message);if(kind==0x000F&&completed!=null)completed();}
  public void Dispose(){if(Handle!=IntPtr.Zero)ReleaseHandle();}
 }
 sealed class Probe:IDisposable {
  internal Sample Current;internal readonly List<Dictionary<string,object>> Attempts=new List<Dictionary<string,object>>();
  readonly ExperimentShell shell;internal readonly ExperimentBridge Bridge;internal readonly ExperimentInput Input;
  internal readonly NativeRendererLifecycle Renderer;readonly FieldInfo presentField,renderField,twistField;
  readonly Action originalPresent;readonly Func<int> originalRender;readonly Func<ExperimentTwist,Task> originalTwist;
  long renderStart,renderReturn;
  internal Probe(ExperimentShell shell){
   this.shell=shell;Bridge=Field<ExperimentBridge>(shell,"bridge");Input=Field<ExperimentInput>(shell,"input");Renderer=Field<NativeRendererLifecycle>(Bridge,"renderer");
   presentField=Renderer.GetType().GetField("present",Flags);renderField=Renderer.GetType().GetField("renderScene",Flags);twistField=Input.GetType().GetField("onTwist",Flags);
   originalPresent=(Action)presentField.GetValue(Renderer);originalRender=(Func<int>)renderField.GetValue(Renderer);originalTwist=(Func<ExperimentTwist,Task>)twistField.GetValue(Input);
   Require(originalPresent!=null&&originalRender!=null&&originalTwist!=null,"Retained native submission/input delegates unavailable");
   try{renderField.SetValue(Renderer,(Func<int>)delegate{renderStart=Tick();int result=originalRender();renderReturn=Tick();return result;});
   presentField.SetValue(Renderer,(Action)delegate{
    long start=Tick();originalPresent();long end=Tick();var sample=Current;if(sample==null)return;
    // Snapshot and Field cannot change mid-callback on the owning UI thread.
    // Copy after the timestamp; compare outside the timed section.
    var field=(short[])Reflect.Get(Field<object>(Bridge,"puzzle"),"Field");var bytes=new byte[field.Length*2];Buffer.BlockCopy(field,0,bytes,0,bytes.Length);
    var owner=Bridge.Viewport.FindForm();sample.Frames.Add(new Frame{RenderStart=renderStart,RenderReturn=renderReturn,PresentStart=start,PresentReturn=end,Snapshot=Bridge.Snapshot,Field=bytes,Foreground=owner!=null&&GetForegroundWindow()==owner.Handle,Visible=Renderer.Subset.VisibleCount,Drawn=Renderer.Subset.LastDrawCount,Motion=Renderer.Subset.LastDrawCount<Renderer.Subset.VisibleCount});
   });
   twistField.SetValue(Input,(Func<ExperimentTwist,Task>)(turn=>{var sample=Current;var task=ObserveTwist(turn);if(sample!=null)sample.TwistTask=task;return task;}));}catch{presentField.SetValue(Renderer,originalPresent);renderField.SetValue(Renderer,originalRender);twistField.SetValue(Input,originalTwist);throw;}
  }
  async Task ObserveTwist(ExperimentTwist turn){var sample=Current;if(sample!=null)sample.Dispatch=Tick();Task task=originalTwist(turn);await task;if(sample!=null){sample.Adopt=Tick();var accepted=task as Task<bool>;sample.Accepted=accepted!=null&&accepted.Result;Require(sample.Accepted,"Measured explicit Twist was rejected");}}
  internal Sample Begin(string kind,string action,int number,int pair,bool warmup){Require(Current==null,"Latency samples cannot overlap");return Current=new Sample{Kind=kind,Action=action,Number=number,Pair=pair,Warmup=warmup,BusyAtEntry=!shell.IsReady,Start=Tick()};}
  internal void End(){if(Current!=null){Attempts.Add(LocalApi.D("kind",Current.Kind,"number",Current.Number,"warmup",Current.Warmup,"input_ticks",Current.Start,"busy_at_entry",Current.BusyAtEntry,"accepted",Current.Accepted,"status",Current.Measured?"measured":Current.Accepted?"accepted-without-correct-surface":Current.Dispatch==0?"not-dispatched":"rejected-or-incomplete"));Current=null;}}
  public void Dispose(){End();presentField.SetValue(Renderer,originalPresent);renderField.SetValue(Renderer,originalRender);twistField.SetValue(Input,originalTwist);}
 }
 static async Task Wait(Func<bool> predicate,string message){long start=Tick();while(!predicate()){if(Milliseconds(start,Tick())>5000)throw new TimeoutException(message);await Task.Delay(2);}}
 static async Task Send(ExperimentShell shell,Func<Task> ready,Dictionary<string,object> body){Require(await shell.Send(body),"Untimed latency fixture action rejected: "+Text(body["action"]));await ready();}
 static async Task<byte[]> Labels(LocalApi api){return await Task.Factory.StartNew(()=>api.Bytes("labels"));}
 static async Task CheckLabels(LocalApi api,byte[] expected,string message){byte[] actual=await Labels(api);Require(actual.Length==259800*4&&actual.SequenceEqual(expected),message);}
 static string Stable(ExperimentShell shell,LocalApi api){var w=Work(shell);return api.Json(new object[]{w["current"],w["target"],w["next"],w["draft"],w["roles"],w["block"],w["goal"]});}
 static async Task ReleaseNewProtection(ExperimentShell shell,Func<Task> ready,List<object> releases,int pair){var orbits=Items(shell.Work["protected"]);if(orbits.Length==0)return;releases.Add(new{afterPair=pair,orbits=orbits});await Send(shell,ready,LocalApi.D("action","protect","orbits",new object[0]));}
 static string Route(ExperimentShell shell,string axis){var state=(ExperimentInputState)typeof(ExperimentShell).GetMethod("InputState",Flags).Invoke(shell,null);var route=state.TwistKeys.Where(p=>p.Value==axis&&!state.GripKeys.ContainsKey(p.Key)&&!state.CommandKeys.ContainsKey(p.Key)&&!state.CommandKeys.ContainsKey("Shift+"+p.Key)).OrderBy(p=>p.Key,StringComparer.Ordinal).Select(p=>p.Key).FirstOrDefault();Require(route!=null,"No unambiguous forward/inverse keyboard route for "+axis);return route;}
 static Dictionary<string,object> Row(Sample sample,NativeSnapshot adopted,Frame frame){
  long end=frame!=null?frame.PresentReturn:sample.Paint;
  Require(sample.Dispatch>=sample.Start&&sample.Adopt>=sample.Dispatch&&end>=sample.Start,"Incomplete or inconsistent latency timestamps");
  sample.Measured=true;return LocalApi.D("kind",sample.Kind,"action",sample.Action,"number",sample.Number,"pair",sample.Pair,"warmup",sample.Warmup,"input_ticks",sample.Start,"dispatch_ticks",sample.Dispatch,"adoption_complete_ticks",sample.Adopt,"correct_surface_ticks",end,"input_to_surface_ms",Milliseconds(sample.Start,end),"dispatch_to_adoption_ms",Milliseconds(sample.Dispatch,sample.Adopt),"adoption_to_surface_ms",Milliseconds(sample.Adopt,end),"state_hash",adopted.State["state_hash"],"head",adopted.State["head"],"snapshot_revision",adopted.Revision,"paint_binding",sample.PaintBinding,"render_start_ticks",frame==null?(object)null:frame.RenderStart,"render_return_ticks",frame==null?(object)null:frame.RenderReturn,"present_start_ticks",frame==null?(object)null:frame.PresentStart,"present_return_ticks",frame==null?(object)null:frame.PresentReturn,"visible_slots",frame==null?(object)null:frame.Visible,"drawn_slots",frame==null?(object)null:frame.Drawn,"motion_policy_active",frame!=null&&frame.Motion);
 }
 static void Append(string path,List<Dictionary<string,object>> rows,Dictionary<string,object> row){rows.Add(row);File.AppendAllText(path,new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(row)+Environment.NewLine);}
 static void EarlyDiagnosis(List<Dictionary<string,object>> rows,string kind){var first=rows.Where(r=>Text(r["kind"])==kind&&!Convert.ToBoolean(r["warmup"])).Take(10).Select(r=>Convert.ToDouble(r["input_to_surface_ms"])).OrderBy(x=>x).ToArray();double bound=kind=="turn"?500:250;Require(first.Length<10||first[9]<=bound,"Initial 10 "+kind+" samples exceed five times the S14 p95 budget; stop as partial diagnostic before an expensive acceptance run");}
 static object Percentiles(IEnumerable<Dictionary<string,object>> source,string kind){var values=source.Where(r=>Text(r["kind"])==kind&&!Convert.ToBoolean(r["warmup"])).Select(r=>Convert.ToDouble(r["input_to_surface_ms"])).OrderBy(x=>x).ToArray();Func<double,double> rank=q=>values[(int)Math.Ceiling(q*values.Length)-1];return new{count=values.Length,median=values.Length==0?(double?)null:rank(.5),p95=values.Length==0?(double?)null:rank(.95),p99=values.Length==0?(double?)null:rank(.99),maximum=values.Length==0?(double?)null:values.Last(),target_ms=kind=="turn"?100:50,target_met=values.Length>=100&&(kind=="turn"?rank(.95)<=100:rank(.95)<50)};}
 // Root must call only in its disposable Session, after recording an explicit
 // fixed legal non-Home setup (for example [1,3]); this method never resets a user solve.
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,string output,Dictionary<string,object> environment){
  Require(Environment.GetEnvironmentVariable("MAGIC600_NATIVE_LATENCY")=="1","Latency run requires the root's explicit disposable-session opt-in");
  await ready();Require(environment!=null&&environment.ContainsKey("recorded_setup"),"Record the actual fixed legal baseline setup in environment");
  var api=Field<LocalApi>(shell,"api");var w=Work(shell);Require(!Object.Equals(Value(Map(shell.Work["session"]),"raw_full_home"),true),"Latency baseline must be non-Home to avoid legitimate completion dialogs");Require(shell.Work["pending"]==null&&Items(shell.Work["protected"]).Length==0&&Items(Value(shell.Work,"position_locks")).Length==0,"Latency fixture requires no pending preview or mechanical protection");
  string baselineHash=Hash(shell),stable=Stable(shell,api),oldBank=Text(w["bank"]),oldInput=Text(w["input"]);var oldCenter=Value(Map(w["view"]),"local_center");byte[] baseline=await Labels(api);Require(baseline.Length==259800*4,"Full baseline labels are unavailable");
  string raw=Path.Combine(output,"native-latency-samples.jsonl"),reportPath=Path.Combine(output,"native-latency.json");Require(!File.Exists(raw)&&!File.Exists(reportPath),"Latency evidence files already exist");var rows=new List<Dictionary<string,object>>();var releases=new List<object>();Exception failure=null;bool restored=false;
  var report=LocalApi.D("status","incomplete","scope","Injected application input / command entry to CPU return from successful native Present or actual WM_PAINT; not physical keyboard, GPU completion, compositor display, or FPS","environment",environment,"clock_frequency",Stopwatch.Frequency,"adoption_boundary","Original Send task completed: paired snapshot adoption plus synchronous Draw; network/parse cannot be separated with the existing API seam","paint_order_note","A synchronous control paint may precede completion of the remaining Draw; negative adoption_to_surface_ms records that real ordering","os",Environment.OSVersion.ToString(),"framework",Environment.Version.ToString(),"processor_count",Environment.ProcessorCount,"baseline_hash",baselineHash,"inverse_pairs",50,"samples",rows,"untimed_protection_releases",releases);
  try{
   await Send(shell,ready,LocalApi.D("action","bank","id","33-A"));await Send(shell,ready,LocalApi.D("action","settings","input","live"));Invoke(shell,"windows-hide");Invoke(shell,"puzzle");await ready();
   using(var probe=new Probe(shell)){report["attempts"]=probe.Attempts;
    var viewport=probe.Bridge.Viewport;var viewportOwner=viewport.FindForm();Require(viewportOwner!=null&&!viewportOwner.IsDisposed,"Actual viewport owner is unavailable");viewportOwner.Activate();viewport.Focus();await Task.Delay(250);Foreground(viewport);using(var graphics=viewport.CreateGraphics())report["native_surface"]=new{bounds=Bounds(viewport),dpiX=graphics.DpiX,dpiY=graphics.DpiY,screen=Screen.FromControl(viewport).Bounds,mode=Field<string>(shell,"mode"),visible_slots=probe.Renderer.Subset.VisibleCount,renderer="retained MPUlt/Managed DirectX",frameworkHidden=probe.Renderer.Subset.HideFramework};
    int turnNumber=0;
    var state=(ExperimentInputState)typeof(ExperimentShell).GetMethod("InputState",Flags).Invoke(shell,null);var caps=state.GripKeys.OrderBy(p=>p.Key,StringComparer.Ordinal).Select(p=>p.Value).Distinct().Take(2).ToArray();Require(caps.Length==2,"Turn set has fewer than two explicit captured caps");report["turn_caps"]=caps;report["turn_axes"]=Axes;report["turn_bank"]=state.BankId;report["turn_frames"]=Value(Items(shell.Work["banks"]).Select(Map).Single(b=>Text(b["id"])==state.BankId),"frames");
    for(int pair=-2;pair<50;pair++){
     int choice=pair<0?pair+2:pair%14,cap=caps[choice<7?0:1];string axis=Axes[choice%7];string code=Route(shell,axis);bool warmup=pair<0;
     Require(probe.Input.SetPointerGrip(cap),"Explicit fixed Grip was rejected");
     for(int direction=0;direction<2;direction++){
      await ready();Foreground(viewport);Require(!probe.Input.Pending,"Input still pending before next timed turn");
      var sample=probe.Begin("turn","C"+cap+" "+axis+(direction==1?" inverse":""),turnNumber++,pair,warmup);
      try{
       Require(probe.Input.HandleKeyDown(code,direction==1,false,false,false,false),"Measured key was not handled");probe.Input.HandleKeyUp(code);
       Require(sample.TwistTask!=null,"Measured key did not dispatch its Twist");await sample.TwistTask;Require(sample.Adopt!=0&&sample.Accepted,"Twist adoption was not accepted");
       Require(!Object.Equals(Value(Map(shell.Work["session"]),"raw_full_home"),true),"Measured turn reached full Home; completion flow invalidates this latency series");var adopted=probe.Bridge.Snapshot;
       await Wait(()=>sample.Frames.Any(f=>Object.ReferenceEquals(f.Snapshot,adopted)&&f.Drawn==f.Visible),"No normal-scheduler frame submitted for the adopted turn");var frame=sample.Frames.First(f=>Object.ReferenceEquals(f.Snapshot,adopted)&&f.Drawn==f.Visible);Foreground(viewport);
       Require(frame.Foreground&&frame.Field.SequenceEqual(adopted.Colors),"Submitted native Field does not equal the complete adopted color mapping");Require(Text(adopted.State["state_hash"])==Hash(shell),"Submitted native frame is not the current authoritative state");Append(raw,rows,Row(sample,adopted,frame));
      }finally{probe.End();probe.Input.HandleKeyUp(code);}
      await ready();
     }
     await CheckLabels(api,baseline,"Legal inverse pair did not restore all baseline labels");Require(Hash(shell)==baselineHash&&Stable(shell,api)==stable,"Inverse pair changed fixed working identity/draft or failed to restore the baseline hash");await ReleaseNewProtection(shell,ready,releases,pair);if(pair==4)EarlyDiagnosis(rows,"turn");
    }
    probe.Input.ReleasePointerGrip();check(true,"50 explicit legal inverse pairs (100 measured turns) restore every baseline label and fixed work context");
    Invoke(shell,"keyboard");await ready();var keyboard=Field<Form>(shell,"keyboardWindow");Require(keyboard!=null&&keyboard.Visible,"Onscreen keyboard must be visible for bank feedback measurement");keyboard.Activate();var caption=Field<Label>(shell,"keyboardContext");await Task.Delay(150);report["bank_surface"]=new{bounds=Bounds(caption),window=Bounds(keyboard)};
    string wantedBank=null;using(var receipt=new PaintReceipt(caption,delegate{var s=probe.Current;if(s!=null&&(s.Paint==0||!Object.ReferenceEquals(s.PaintedSnapshot,probe.Bridge.Snapshot))&&s.Kind=="bank"&&Text(Work(shell)["bank"])==wantedBank&&caption.Text.StartsWith("SET "+wantedBank+" ·",StringComparison.Ordinal)){s.Paint=Tick();s.PaintedSnapshot=probe.Bridge.Snapshot;s.PaintBinding=wantedBank;}})){
     for(int i=-2;i<100;i++){wantedBank=(i&1)==0?"Keyboard":"Macro";await ready();Foreground(caption);var sample=probe.Begin("bank",wantedBank,i,i/2,i<0);try{sample.Dispatch=Tick();sample.Accepted=await shell.Send(LocalApi.D("action","bank","id",wantedBank));Require(sample.Accepted,"Measured bank selection rejected");sample.Adopt=Tick();var adopted=probe.Bridge.Snapshot;await Wait(()=>sample.Paint>sample.Start&&Object.ReferenceEquals(sample.PaintedSnapshot,adopted),"New bank has no matching actual paint receipt");Foreground(caption);Append(raw,rows,Row(sample,adopted,null));}finally{probe.End();}if(i==9)EarlyDiagnosis(rows,"bank");}
    }
    await CheckLabels(api,baseline,"Bank changes altered full mechanical state");check(true,"100 bank changes each produce a fresh matching onscreen keyboard paint without changing labels");
    Invoke(shell,"local");await ready();var local=Field<NativeCellView>(shell,"local");var localWindow=local.FindForm();localWindow.Activate();await Send(shell,ready,LocalApi.D("action","settings","view",LocalApi.D("local_center",7)));await Task.Delay(150);report["local_surface"]=new{bounds=Bounds(local),window=Bounds(localWindow)};int wantedCell=0;
    using(var receipt=new PaintReceipt(local,delegate{var s=probe.Current;if(s!=null&&(s.Paint==0||!Object.ReferenceEquals(s.PaintedSnapshot,probe.Bridge.Snapshot))&&s.Kind=="local-center"&&local.CenterCell==wantedCell&&Field<int>(local,"cutCell")==wantedCell&&local.LocalStateHash==Hash(shell)){s.Paint=Tick();s.PaintedSnapshot=probe.Bridge.Snapshot;s.PaintBinding="C"+wantedCell;}})){
     for(int i=-2;i<100;i++){wantedCell=(i&1)==0?1:7;await ready();Foreground(local);var sample=probe.Begin("local-center","C"+wantedCell,i,i/2,i<0);try{sample.Dispatch=Tick();sample.Accepted=await shell.Send(LocalApi.D("action","settings","view",LocalApi.D("local_center",wantedCell)));Require(sample.Accepted,"Measured Local center change rejected");sample.Adopt=Tick();var adopted=probe.Bridge.Snapshot;await Wait(()=>sample.Paint>sample.Start&&Object.ReferenceEquals(sample.PaintedSnapshot,adopted),"New Local center has no matching actual paint receipt");Foreground(local);int[] labels=Field<int[]>(local,"cutLabels");Require(labels.Length==433&&labels.Select((label,index)=>(uint)label==BitConverter.ToUInt32(baseline,((wantedCell-1)*433+index)*4)).All(value=>value),"Painted Local cell does not contain its 433 actual labels");Append(raw,rows,Row(sample,adopted,null));}finally{probe.End();}if(i==9)EarlyDiagnosis(rows,"local-center");}
    }
    await CheckLabels(api,baseline,"Local center changes altered full mechanical state");check(true,"100 Local center changes each paint the actual 433 labels without changing the puzzle");
   }
  }catch(Exception error){failure=error;report["failure_type"]=error.GetType().FullName;report["failure_message"]=error.Message;}
  // Never repair a failed mechanical sample with a reset or guessed inverse.
  try{await ready();await CheckLabels(api,baseline,"Final full state differs from the recorded non-Home baseline");await ReleaseNewProtection(shell,ready,releases,50);await Send(shell,ready,LocalApi.D("action","bank","id",oldBank));await Send(shell,ready,LocalApi.D("action","settings","input",oldInput,"view",LocalApi.D("local_center",oldCenter)));restored=Hash(shell)==baselineHash&&Stable(shell,api)==stable;Require(restored,"Latency fixture restoration changed the original work context");}catch(Exception cleanup){failure=failure==null?cleanup:new AggregateException(failure,cleanup);report["restoration_failure"]=cleanup.Message;}
  report["status"]=failure==null?"measured":"failed";report["baseline_restored"]=restored;report["percentiles"]=new[]{Percentiles(rows,"turn"),Percentiles(rows,"bank"),Percentiles(rows,"local-center")};File.WriteAllText(reportPath,new JavaScriptSerializer{MaxJsonLength=64000000,RecursionLimit=128}.Serialize(report));if(failure!=null)throw failure;
 }
}
