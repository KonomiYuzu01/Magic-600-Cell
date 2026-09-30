// Actual native controls over an isolated legal E1 witness; not a human solving trial.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class MacroVariantsNativeChecks {
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static object[] Items(object value){return (object[])value;}
 static string Json(object value){return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(value);}
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static IEnumerable<Control> Children(Control parent){foreach(Control child in parent.Controls){yield return child;foreach(var nested in Children(child))yield return nested;}}
 static string Id(object item){return Convert.ToString(item.GetType().GetField("Id",BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.Public).GetValue(item));}
 static Dictionary<string,object> Record(ExperimentShell shell,string id){return Items(shell.Work["library"]).Select(Map).Single(row=>Convert.ToString(row["id"])==id);}
 static string Context(ExperimentShell shell){var work=Map(shell.Work["workspace"]);return Json(new object[]{shell.Work["hash"],work["current"],work["target"],work["next"],work["bank"],work["draft"],work["draft_sources"],work["block"],work["roles"],work["reference"],shell.Work["pending"]});}
 static void Invoke(ExperimentShell shell,string id){typeof(ExperimentShell).GetMethod("RunCommand",BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,new object[]{id});}
 static async Task<Form> Open(ExperimentShell shell,Form owner,string command,string title){owner.BeginInvoke((Action)(async delegate{while(!shell.IsReady)await Task.Delay(30);Invoke(shell,command);}));var deadline=DateTime.UtcNow.AddSeconds(25);while(DateTime.UtcNow<deadline){var dialog=Application.OpenForms.Cast<Form>().FirstOrDefault(f=>f.Visible&&f.Text==title);if(dialog!=null)return dialog;await Task.Delay(30);}throw new TimeoutException("Dialog did not open: "+title+" / "+Field<Label>(shell,"feedback").Text);}
 static Button Button(Form form,string text){return Children(form).OfType<Button>().Single(b=>b.Text==text);}
 static void Close(Form form){((IButtonControl)form.CancelButton).PerformClick();}
 static bool VisibleInside(Control control){if(!control.Visible||!control.IsHandleCreated)return false;var rect=control.RectangleToScreen(control.ClientRectangle);var visible=rect;for(Control parent=control.Parent;parent!=null;parent=parent.Parent)visible=Rectangle.Intersect(visible,parent.RectangleToScreen(parent.ClientRectangle));return visible==rect;}
 static void Select(ComboBox list,string id){list.SelectedItem=list.Items.Cast<object>().Single(item=>Id(item)==id);}
 static async Task Pick(ExperimentShell shell,string id,Func<Task> ready){var list=Field<ListBox>(shell,"macros");list.SelectedItem=list.Items.Cast<object>().Single(item=>Id(item)==id);await ready();}

 internal static async Task<string> Run(ExperimentShell shell,Form owner,Func<Task> ready,Func<Dictionary<string,object>,Task<bool>> send,Action<bool,string> check,Action<string,Form> image){
  await Pick(shell,"o33-n0-forward",ready);string source=Field<string>(shell,"selectedMacro"),before=Context(shell);string original=Json(Record(shell,source));
  var details=await Open(shell,owner,"macro-compare","Macro details");
  var tabs=Children(details).OfType<TabControl>().Single();check(tabs.SelectedTab.Text=="Compare","Compare command opens its named tab without switching the work bank");
  var second=Children(details).OfType<ComboBox>().Single(c=>c.AccessibleName=="Second macro to compare");check(second.SelectedIndex==0&&Id(second.SelectedItem)==""&&!Button(details,"Check comparison").Enabled,"No second macro is silently chosen for comparison");
  Select(second,"o33-n0-inverse");Button(details,"Check comparison").PerformClick();await ready();
  var comparison=Field<Dictionary<string,object>>(shell,"lastCommandResult");check(Convert.ToBoolean(comparison["inverse"])&&!Convert.ToBoolean(comparison["full_equal"]),"Explicit forward/inverse pair has exact reverse action, not whole equality");
  check(Convert.ToString(Map(Map(comparison["a"])["record"])["id"])==source,"Comparison carries the selected canonical source record");
  check(Convert.ToString(comparison["prefix_protection"])=="not_checked","Net comparison does not certify intermediate protection");
  details.Size=new Size(640,520);await Task.Delay(100);check(VisibleInside(second)&&VisibleInside(Button(details,"Check comparison")),"Compact comparison keeps both explicit choice and check action visible");image("14-macro-compare",details);Close(details);await ready();
  check(Context(shell)==before&&Field<string>(shell,"selectedMacro")==source,"Closing comparison preserves Current, Next, phases, bank and all labels");

  var save=await Open(shell,owner,"macro-inverse","Save inverse macro");Children(save).OfType<TextBox>().Single(c=>c.AccessibleName=="New macro name").Text="Cancelled variant";Close(save);await ready();
  check(Context(shell)==before&&!Items(shell.Work["library"]).Select(Map).Any(row=>Convert.ToString(row["name"])=="Cancelled variant"),"Cancel before saving leaves no macro or workspace change");
  save=await Open(shell,owner,"macro-inverse","Save inverse macro");var name=Children(save).OfType<TextBox>().Single(c=>c.AccessibleName=="New macro name");name.Text="First insertion — explicitly saved inverse";
  Button(save,"Save inverse").PerformClick();await ready();var result=Field<Dictionary<string,object>>(shell,"lastCommandResult");string saved=Convert.ToString(result["created_id"]);
  check(saved!=source&&Record(shell,saved)!=null,"Saving produces a new canonical library entry");
  check(Field<string>(shell,"selectedMacro")==source&&Context(shell)==before,"Saving an inverse does not select, insert or execute it");
  check(Json(Record(shell,source))==original,"Saving an inverse preserves the source record and original steps");
  check(Convert.ToString(Map(Record(shell,saved)["derived_from"])["id"])==source,"Saved inverse retains source identity and derivation");
  save.Size=new Size(640,520);await Task.Delay(100);check(VisibleInside(Button(save,"Select saved macro")),"Compact saved result exposes a separate explicit selection action");image("15-macro-saved-inverse",save);
  Button(save,"Select saved macro").PerformClick();await ready();check(Field<string>(shell,"selectedMacro")==saved&&Context(shell)==before,"Explicit saved selection changes only selected macro and read-only inspection");

  details=await Open(shell,owner,"macro-compare","Macro details");second=Children(details).OfType<ComboBox>().Single(c=>c.AccessibleName=="Second macro to compare");Select(second,"o33-n0-inverse");Button(details,"Check comparison").PerformClick();await ready();comparison=Field<Dictionary<string,object>>(shell,"lastCommandResult");
  check(Convert.ToBoolean(comparison["full_equal"]),"Created inverse exactly equals the explicitly chosen retained insertion witness");Close(details);await ready();
  check(await send(LocalApi.D("action","reference","word",new[]{1})),"Explicit legal R word selected");await ready();string rContext=Context(shell);
  save=await Open(shell,owner,"reference-transform","Save macro with R");check(Children(save).OfType<TextBoxBase>().Any(c=>c.AccessibleName=="Explicit reference word"&&c.Text.Contains("1")),"R variant displays the actual chosen finite word");
  Children(save).OfType<TextBox>().Single(c=>c.AccessibleName=="New macro name").Text="Chosen R variant — inspect only";Button(save,"Save with R").PerformClick();await ready();result=Field<Dictionary<string,object>>(shell,"lastCommandResult");string rId=Convert.ToString(result["created_id"]);
  var provenance=Map(Record(shell,rId)["derived_from"]);check(Convert.ToString(provenance["convention"])=="R^-1 / Macro / R"&&Json(provenance["reference"])=="[1]","Chosen-word variant preserves exact reference and chronological convention");
  check(Field<string>(shell,"selectedMacro")==saved&&Context(shell)==rContext,"Saving R variant retains the previously chosen solving macro and all work");image("16-macro-r-variant",save);Close(save);await ready();
  check(await send(LocalApi.D("action","reference","word",new int[0])),"Explicitly restore canonical R for the fixed E1 insertion");await ready();check(Context(shell)==before,"Variant exploration leaves the original solving context intact");
  check(Field<string>(shell,"selectedMacro")==saved,"Closing R result does not replace the explicitly selected inverse");return saved;
 }

 internal static async Task PendingClose(ExperimentShell shell,Form owner,Func<Task> ready,Action<bool,string> check){
  string context=Context(shell),selected=Field<string>(shell,"selectedMacro");
  var details=await Open(shell,owner,"macro-compare","Macro details");
  var second=Children(details).OfType<ComboBox>().Single(c=>c.AccessibleName=="Second macro to compare");Select(second,"o33-n0-inverse");Button(details,"Check comparison").PerformClick();await ready();
  check(Context(shell)==context&&Convert.ToBoolean(shell.Work["executable"]),"Actual comparison preserves the unrelated staged preview and its execution context");
  Button(details,"Check comparison").PerformClick();var stop=Button(details,"Stop check");check(stop.Enabled,"A running native comparison keeps Stop check reachable");stop.PerformClick();await ready();
  check(Context(shell)==context&&Convert.ToBoolean(shell.Work["executable"]),"Stop request or already-finished comparison preserves staged work; neither executes it");Close(details);await ready();
  var save=await Open(shell,owner,"macro-inverse","Save inverse macro");Close(save);await ready();
  check(Context(shell)==context&&Field<string>(shell,"selectedMacro")==selected,"Closing comparison and unsaved variant preserves an unrelated staged preview and its work");
 }
}
