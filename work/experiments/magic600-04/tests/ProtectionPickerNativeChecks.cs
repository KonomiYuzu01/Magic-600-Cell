// Agent-operated Windows controls; isolated fixture, not a physical keyboard trial.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class ProtectionPickerNativeChecks {
 [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
 static T Field<T>(object owner,string name){return (T)owner.GetType().GetField(name,BindingFlags.Instance|BindingFlags.NonPublic|BindingFlags.Public).GetValue(owner);}
 static void Set(ExperimentShell shell,string name,object value){typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).SetValue(shell,value);}
 static void Call(ExperimentShell shell,string name,params object[] args){typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,args);}
 static IEnumerable<Control> All(Control parent){foreach(Control child in parent.Controls){yield return child;foreach(Control next in All(child))yield return next;}}
 static int Id(object choice){return Int32.Parse(Field<string>(choice,"Id"));}
 static int[] Protected(ExperimentShell shell){return ((System.Collections.IEnumerable)shell.Work["protected"]).Cast<object>().Select(Convert.ToInt32).OrderBy(id=>id).ToArray();}
 static string Context(ExperimentShell shell){var w=(Dictionary<string,object>)shell.Work["workspace"];return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(new object[]{shell.Work["hash"],w["current"],w["next"],w["target"],w["roles"],w["draft"],w["reference"],w["block"],w["bank"]});}
 static async Task Dialog(ExperimentShell shell,Func<Form,Task> visit){
  var done=new TaskCompletionSource<bool>();using(var timer=new Timer{Interval=60}){
   timer.Tick+=async delegate{timer.Stop();Form dialog=null;try{dialog=Application.OpenForms.Cast<Form>().Single(f=>f.Text=="Orbit protection");await visit(dialog);done.SetResult(true);}catch(Exception error){done.SetException(error);}finally{if(dialog!=null&&!dialog.IsDisposed)dialog.Close();}};
   timer.Start();try{Call(shell,"RunCommand","protection");}finally{timer.Stop();}if(await Task.WhenAny(done.Task,Task.Delay(30000))!=done.Task)throw new Exception("Protection editor callback did not finish");await done.Task;
  }
 }
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();check(Convert.ToInt32(((Dictionary<string,object>)shell.Work["workspace"])["orbit"])==33,"Protection picker uses the caller's disclosed legal E1 fixture");string before=Context(shell);var api=Field<LocalApi>(shell,"api");byte[] labels=await Task.Factory.StartNew(()=>api.Bytes("labels"));
  Call(shell,"RunCommand","solve-protection");await ready();var solve=Field<Dictionary<string,Form>>(shell,"workWindows")["solve"];var bounds=solve.Bounds;
  solve.Size=solve.MinimumSize;await Task.Delay(100);var table=Field<TableLayoutPanel>(shell,"solveProtection");
  check(table.GetRow(Field<ListBox>(shell,"solveProtectionList"))==3,"Protection actions precede the elastic list");
  var boundary=Field<Dictionary<Button,string>>(shell,"commandButtons").Single(p=>p.Value=="protection"&&table.Contains(p.Key)).Key;
  check(boundary.Visible&&boundary.RectangleToScreen(boundary.ClientRectangle).Top< Field<ListBox>(shell,"solveProtectionList").RectangleToScreen(Field<ListBox>(shell,"solveProtectionList").ClientRectangle).Top,"Boundary action is above the declared requirements, including an empty set");image("protection-actions-minimum",solve);
  solve.Bounds=bounds;
  await Dialog(shell,async dialog=>{
   var list=All(dialog).OfType<CheckedListBox>().Single();var apply=All(dialog).OfType<Button>().Single(b=>b.Text=="Apply boundary");
   check(list.Items.Count==35&&list.Items.Cast<object>().Select(Id).Distinct().Count()==35,"All35 mathematical labels map to distinct canonical moving orbits");
   check(list.CheckedItems.Count==0&&Id(list.SelectedItem)==33,"Working orbit starts focused, not silently protected");
   check(list.Items.Cast<object>().All(x=>!x.ToString().StartsWith("Orbit ",StringComparison.Ordinal)),"Named rows use model-derived mathematical descriptions");
   image("protection-named-picker",dialog);check(dialog.ContainsFocus&&list.Focused,"Editor gives native focus to the named list");
   // Native keyboard dispatch is guarded by this owned modal's actual focus.
   check(Form.ActiveForm==dialog&&GetForegroundWindow()==dialog.Handle,"Owned picker is foreground before generated Space input");SendKeys.SendWait(" ");await Task.Delay(30);
   check(list.CheckedItems.Count==1&&Id(list.CheckedItems[0])==33,"Space checks exactly the focused canonical orbit");apply.PerformClick();await ready();
  });
  check(Protected(shell).SequenceEqual(new[]{33})&&Context(shell)==before,"Apply changes only the declared orbit boundary; Current, locked Next and draft are retained");
  await Dialog(shell,async dialog=>{var list=All(dialog).OfType<CheckedListBox>().Single();int row=Enumerable.Range(0,list.Items.Count).Single(i=>Id(list.Items[i])==0);list.SetItemChecked(row,true);All(dialog).OfType<Button>().Single(b=>b.Text=="Cancel").PerformClick();await Task.Delay(20);});
  check(Protected(shell).SequenceEqual(new[]{33}),"Cancel discards local checkbox edits");
  await Dialog(shell,async dialog=>{
   var apply=All(dialog).OfType<Button>().Single(b=>b.Text=="Apply boundary");Set(shell,"stopPending",true);try{apply.PerformClick();await Task.Delay(20);check(!dialog.IsDisposed&&Protected(shell).SequenceEqual(new[]{33}),"Stop acknowledgement blocks protection changes and keeps the editor open");}finally{Set(shell,"stopPending",false);}
   object actual=shell.Work["protected"];shell.Work["protected"]=new[]{0};try{apply.PerformClick();await Task.Delay(20);check(!dialog.IsDisposed&&All(dialog).OfType<Label>().Any(l=>l.Text.StartsWith("Protection changed.",StringComparison.Ordinal)),"A changed full boundary rejects stale Apply without overwriting it");}finally{shell.Work["protected"]=actual;}
  });
  await Dialog(shell,async dialog=>{var list=All(dialog).OfType<CheckedListBox>().Single();for(int i=0;i<list.Items.Count;i++)list.SetItemChecked(i,false);All(dialog).OfType<Button>().Single(b=>b.Text=="Apply boundary").PerformClick();await ready();});
  check(Protected(shell).Length==0&&Context(shell)==before,"An explicitly empty boundary is supported without changing solving work");
  byte[] after=await Task.Factory.StartNew(()=>api.Bytes("labels"));check(labels.SequenceEqual(after),"Selection, cancellation and protection edits preserve all259800 actual labels");
  check(!Field<bool>(shell,"modal")&&!Field<bool>(shell,"stopPending"),"Editor closes without retained modal or Stop input ownership");image("protection-picker-complete",solve);
 }
}
