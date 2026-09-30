// Root-operated actual native display integration. No physical-input or GPU claim.
// Requires a disposable Session with a real staged preview; preserves that work.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class DisplayNativeChecks {
 const BindingFlags Flags=BindingFlags.Instance|BindingFlags.Public|BindingFlags.NonPublic;
 static readonly string[] Names={"trk_faceShrink","trk_StickerSize","trk_ViewAngle","trk_LightDiff","trk_LightSpec","trk_LightAmb"};
 static readonly string[] Labels={"Cell size","Sticker size","Field of view","Diffuse light","Specular light","Ambient light"};
 static T Field<T>(object instance,string name){return (T)instance.GetType().GetField(name,Flags).GetValue(instance);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static object Value(Dictionary<string,object> map,string key){object value;return map!=null&&map.TryGetValue(key,out value)?value:null;}
 static object Ordered(object value){var map=value as IDictionary<string,object>;if(map!=null)return new SortedDictionary<string,object>(map.ToDictionary(p=>p.Key,p=>Ordered(p.Value)),StringComparer.Ordinal);var array=value as IEnumerable;if(array!=null&&!(value is string))return array.Cast<object>().Select(Ordered).ToArray();return value;}
 static string Json(object value){return new JavaScriptSerializer{MaxJsonLength=64000000,RecursionLimit=128}.Serialize(Ordered(value));}
 static IEnumerable<Control> Children(Control root){foreach(Control c in root.Controls){yield return c;foreach(Control child in Children(c))yield return child;}}
 static void Invoke(ExperimentShell shell,string command){typeof(ExperimentShell).GetMethod("RunCommand",Flags).Invoke(shell,new object[]{command});}
 static string Stable(ExperimentShell shell){return Json(new object[]{shell.Work["hash"],shell.Work["head"],shell.Work["revision"],shell.Work["guard"],shell.Work["review_context"],shell.Work["workspace"],shell.Work["review"],shell.Work["pending"],shell.Work["protected"],shell.Work["position_locks"],shell.Work["executable"]});}
 static string WorkExceptFilter(ExperimentShell shell){var w=new Dictionary<string,object>(Map(shell.Work["workspace"]));w.Remove("filter");return Json(new object[]{shell.Work["hash"],shell.Work["head"],shell.Work["revision"],w,shell.Work["pending"],shell.Work["protected"],shell.Work["position_locks"]});}
 static string InputMap(ExperimentShell shell){var state=(ExperimentInputState)typeof(ExperimentShell).GetMethod("InputState",Flags).Invoke(shell,null);return Json(new object[]{state.BankId,state.GripMode,state.Destination,state.Phase,state.FrameKey,state.GripKeys,state.TwistKeys,state.CommandKeys});}
 static async Task<byte[]> FullLabels(ExperimentShell shell){var api=Field<LocalApi>(shell,"api");return await Task.Factory.StartNew(()=>api.Bytes("labels"));}
 static async Task Send(ExperimentShell shell,Func<Task> ready,Action<bool,string> check,string action,params object[] fields){var data=LocalApi.D(fields);data["action"]=action;check(await shell.Send(data),"Display fixture accepted "+action);await ready();}
 static async Task<Form> Open(ExperimentShell shell,Form host,Func<Task> ready){await ready();host.BeginInvoke((Action)delegate{Invoke(shell,"display");});var deadline=DateTime.UtcNow.AddSeconds(30);while(DateTime.UtcNow<deadline){var dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text=="Puzzle display");if(dialog!=null&&Form.ActiveForm==dialog)return dialog;await Task.Delay(25);}throw new TimeoutException("Actual Puzzle display dialog did not activate");}
 static async Task Saved(ExperimentShell shell,Form dialog,Func<Task> ready){await ready();var deadline=DateTime.UtcNow.AddSeconds(30);while(!dialog.IsDisposed&&Children(dialog).OfType<TrackBar>().Any(s=>!s.Enabled)){if(DateTime.UtcNow>deadline)throw new TimeoutException("Display controls did not re-enable after save");await Task.Delay(25);}await ready();}
 static TrackBar Slider(Form dialog,string name){return Children(dialog).OfType<TrackBar>().Single(s=>s.AccessibleName==name);}
 static Label Caption(TrackBar slider){return slider.Parent.Controls.OfType<Label>().Single();}
 static int Different(TrackBar slider){return slider.Value==slider.Minimum?slider.Maximum:slider.Minimum+(slider.Value-slider.Minimum)/2;}
 static string ExpectedCaption(TrackBar slider,int index){return Labels[index]+"  "+((index==2?180.0:100.0)*slider.Value/slider.Maximum).ToString("F0")+(index==2?"°":"%");}
 static bool FullyExposed(Control control){if(!control.Visible||!control.IsHandleCreated)return false;var full=control.RectangleToScreen(control.ClientRectangle);var exposed=full;for(Control parent=control.Parent;parent!=null;parent=parent.Parent)exposed=Rectangle.Intersect(exposed,parent.RectangleToScreen(parent.ClientRectangle));return full==exposed&&Screen.FromControl(control).WorkingArea.Contains(full);}
 static void Dimensions(ExperimentBridge bridge,Action<bool,string> check,string boundary){var form=Field<Form>(bridge,"form");var cube=Field<object>(bridge,"cube");check(Convert.ToDouble(Reflect.Get(cube,"FShr"))==Convert.ToDouble(Reflect.Property(form,"CPShrinkFace"))&&Convert.ToDouble(Reflect.Get(cube,"SShr"))==Convert.ToDouble(Reflect.Property(form,"CPStickerSize")),boundary+" uses the exact original slider dimensions in FShr/SShr");}
 static void Preferences(ExperimentBridge bridge,Action<bool,string> check,bool frame,bool adaptive){var view=Map(Map(bridge.Snapshot.State["prefs"])["view"]);check(bridge.FrameworkHidden==frame&&Object.Equals(Value(view,"native_hide_frame"),frame),"Framework checkbox agrees with actual renderer and persisted snapshot");check(bridge.AdaptiveMotion==adaptive&&Object.Equals(Value(view,"native_adaptive_motion"),adaptive),"Adaptive checkbox agrees with actual lifecycle and persisted snapshot");}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();var bridge=Field<ExperimentBridge>(shell,"bridge");var commands=Field<Dictionary<string,Action>>(shell,"commands");foreach(string id in new[]{"display","frame-toggle","detail-toggle","reset-view"})check(commands.ContainsKey(id),"Display command registered: "+id);
  check(shell.Work["pending"]!=null&&Map(Map(shell.Work["workspace"])["draft"]).Values.Cast<object[]>().Any(a=>a.Length>0),"Display fixture has an actual staged preview and nonempty draft");
  string before=Stable(shell),inputBefore=InputMap(shell),filter=Convert.ToString(Map(shell.Work["workspace"])["filter"]);byte[] labels=await FullLabels(shell);check(labels.Length==259800*4,"Display check reads every authoritative label");
  string originalRules=Json(Map(bridge.Snapshot.State["prefs"])["rules"]);check(filter=="active"&&originalRules==Json(new object[]{LocalApi.D("expr","active","style","solid")}),"Display fixture starts with the explicit single Active solid filter; its full rules can be restored");
  bool originalFrame=bridge.FrameworkHidden,originalAdaptive=bridge.AdaptiveMotion;int[] originalValues=Names.Select(n=>bridge.DisplaySlider(n).Value).ToArray();
  host.Activate();Control prior=Children(host).FirstOrDefault(c=>(c is Button||c is TextBox||c is ComboBox||c is ListBox)&&c.CanFocus);check(prior!=null&&prior.Focus(),"Display fixture establishes a real prior focus target");await Task.Delay(50);
  Form dialog=await Open(shell,host,ready);Exception failure=null;
  try{
   var frame=Children(dialog).OfType<CheckBox>().Single(c=>c.AccessibleName=="Hide 600-cell framework");var adaptive=Children(dialog).OfType<CheckBox>().Single(c=>c.AccessibleName=="Adaptive motion detail");var sliders=Labels.Select(n=>Slider(dialog,n)).ToArray();
   check(frame.Checked==originalFrame&&adaptive.Checked==originalAdaptive&&sliders.Length==6,"Display opens with actual native preferences and six retained sliders");
   for(int i=0;i<sliders.Length;i++){var original=bridge.DisplaySlider(Names[i]);var slider=sliders[i];check(slider.Minimum==original.Minimum&&slider.Maximum==original.Maximum&&slider.Value==original.Value&&slider.TickFrequency==original.TickFrequency&&slider.SmallChange==original.SmallChange&&slider.LargeChange==original.LargeChange,"Retained slider range and increments: "+Labels[i]);((ScrollableControl)slider.Parent.Parent).ScrollControlIntoView(slider.Parent);check(FullyExposed(slider),"Display slider is reachable: "+Labels[i]);slider.Value=Different(slider);check(original.Value==slider.Value&&Caption(slider).Text==ExpectedCaption(slider,i),"Display change reaches original control with accurate caption: "+Labels[i]);}
   Dimensions(bridge,check,"Changed cell/sticker sizes");check(Stable(shell)==before&&InputMap(shell)==inputBefore,"All six visual controls preserve draft, preview, guard, roles and input mappings");
   // Controlled admission failure only. No native engine or response is replaced.
   var busy=typeof(ExperimentShell).GetField("busy",Flags);check(!Field<bool>(shell,"busy"),"Busy rejection starts with no actual request in flight");int old=sliders[0].Value;string caption=Caption(sliders[0]).Text;busy.SetValue(shell,true);
   try{sliders[0].Value=Different(sliders[0]);check(sliders[0].Value==old&&bridge.DisplaySlider(Names[0]).Value==old&&Caption(sliders[0]).Text==caption,"Rejected display input restores the thumb and value caption to the native value");check(Field<Label>(shell,"feedback").Text.Contains("Wait for the current operation"),"Rejected display change gives an explicit refusal");}finally{busy.SetValue(shell,false);}
   frame.Checked=!originalFrame;check(sliders.All(s=>!s.Enabled)&&!adaptive.Enabled,"An actual preference save disables every slider and both toggles before awaiting its response");await Saved(shell,dialog,ready);Preferences(bridge,check,!originalFrame,originalAdaptive);
   adaptive.Checked=!originalAdaptive;check(sliders.All(s=>!s.Enabled)&&!frame.Enabled,"Adaptive save also reserves its display controls");await Saved(shell,dialog,ready);Preferences(bridge,check,!originalFrame,!originalAdaptive);
   check(Stable(shell)==before&&InputMap(shell)==inputBefore,"Saved display booleans preserve the exact staged work, guard and effective input mappings");
   await Send(shell,ready,check,"display-settings","settings",LocalApi.D("hide_frame",!originalFrame,"adaptive_motion",!originalAdaptive));check(Stable(shell)==before,"No-op display save preserves review and execution authority");
   ((ScrollableControl)sliders[0].Parent.Parent).ScrollControlIntoView(sliders[0].Parent);image("display-controls",dialog);((IButtonControl)dialog.CancelButton).PerformClick();await ready();check((dialog.IsDisposed||!dialog.Visible)&&Form.ActiveForm==host&&(prior.Focused||prior.ContainsFocus),"Done restores the owning host and its prior focus target");
   Invoke(shell,"reset-view");await ready();Dimensions(bridge,check,"Reset view");check(Stable(shell)==before,"Reset view leaves the existing pending operation intact");
   // Observe real retained coordinates, then hide/reveal through the real filter.
   var access=Field<NativeStickerAccess>(bridge,"access");var visible=Enumerable.Range(0,bridge.Snapshot.Styles.Length).Where(i=>bridge.Snapshot.Styles[i]!=0).ToArray();check(visible.Length>=5,"Actual filtered view exposes at least five native stickers for Reset observation");int[] slots={visible[0],visible[visible.Length/4],visible[visible.Length/2],visible[3*visible.Length/4],visible[visible.Length-1]};var resetCoords=slots.Select(i=>Json(Reflect.Get(access.Slots[i],"Coords"))).ToArray();string workBeforeFilter=WorkExceptFilter(shell);
   await Send(shell,ready,check,"filter","expression","nothing");check(bridge.Snapshot.Styles.All(s=>s==0),"Explicit Nothing filter really hides all native sticker slots");await Send(shell,ready,check,"filter","expression","all");Dimensions(bridge,check,"Hide/reveal after Reset");
   check(slots.Select(i=>Json(Reflect.Get(access.Slots[i],"Coords"))).SequenceEqual(resetCoords),"Five actually visible native sticker meshes retain Reset coordinates after hide/reveal");check(WorkExceptFilter(shell)==workBeforeFilter&&InputMap(shell)==inputBefore,"Filter observation preserves mechanics, pending work, roles and input mappings");
   await Send(shell,ready,check,"filter","expression",filter);check((await FullLabels(shell)).SequenceEqual(labels),"Display, Reset and filter reapplication preserve all 259800 labels");
  }catch(Exception error){failure=error;}
  // .NET 4 compiler: asynchronous cleanup stays outside finally.
  try{if(dialog!=null&&!dialog.IsDisposed){((IButtonControl)dialog.CancelButton).PerformClick();await ready();}if(shell.IsReady){await Send(shell,ready,check,"display-settings","settings",LocalApi.D("hide_frame",originalFrame,"adaptive_motion",originalAdaptive));for(int i=0;i<Names.Length;i++)bridge.SetDisplayValue(Names[i],originalValues[i]);if(Convert.ToString(Map(shell.Work["workspace"])["filter"])!=filter)await Send(shell,ready,check,"filter","expression",filter);Dimensions(bridge,check,"Restored original display");check(Json(Map(bridge.Snapshot.State["prefs"])["rules"])==originalRules,"Display fixture restores the complete original filter rules");check(InputMap(shell)==inputBefore&&(await FullLabels(shell)).SequenceEqual(labels),"Display fixture cleanup restores original controls without remapping or moving labels");}else throw new InvalidOperationException("Display fixture cannot restore controls because native input is not ready");}catch(Exception cleanup){if(failure!=null)throw new AggregateException(failure,cleanup);throw;}
  if(failure!=null)throw failure;
 }
}

