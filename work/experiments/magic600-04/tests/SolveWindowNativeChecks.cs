// Agent-operated checks of the one-window presentation. Caller owns the native GUI.
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using System.Web.Script.Serialization;
using System.Windows.Forms;

internal static class SolveWindowNativeChecks {
 static T Field<T>(ExperimentShell shell,string name){return (T)typeof(ExperimentShell).GetField(name,BindingFlags.Instance|BindingFlags.NonPublic).GetValue(shell);}
 static void Invoke(ExperimentShell shell,string name,params object[] arguments){typeof(ExperimentShell).GetMethod(name,BindingFlags.Instance|BindingFlags.NonPublic).Invoke(shell,arguments);}
 static Dictionary<string,object> Map(object value){return (Dictionary<string,object>)value;}
 static string Context(ExperimentShell shell){var w=Map(shell.Work["workspace"]);return new JavaScriptSerializer{MaxJsonLength=64000000}.Serialize(new object[]{shell.Work["hash"],shell.Work["current"],shell.Work["next"],shell.Work["pending"],w["draft"],w["roles"],w["reference"],w["bank"],w["target"],w["block"],shell.Work["protected"]});}
 static IEnumerable<Control> Descendants(Control parent){foreach(Control child in parent.Controls){yield return child;foreach(Control nested in Descendants(child))yield return nested;}}
 static bool Exposed(Control control){if(!control.Visible||!control.IsHandleCreated||control.Width<=0||control.Height<=0)return false;var whole=control.RectangleToScreen(control.ClientRectangle);var visible=whole;for(Control parent=control.Parent;parent!=null;parent=parent.Parent)visible=Rectangle.Intersect(visible,parent.RectangleToScreen(parent.ClientRectangle));return whole==Rectangle.Intersect(visible,Screen.FromControl(control).WorkingArea);}
 static string BoundsChain(Control control){var rows=new List<string>();for(Control c=control;c!=null;c=c.Parent){var scroll=c as ScrollableControl;rows.Add(c.GetType().Name+" name="+c.AccessibleName+" bounds="+c.Bounds+" client="+c.ClientRectangle+" screen="+(c.IsHandleCreated?c.RectangleToScreen(c.ClientRectangle).ToString():"no handle")+" preferred="+c.PreferredSize+" padding="+c.Padding+" margin="+c.Margin+(scroll==null?"":" scroll="+scroll.AutoScrollPosition+" minimum="+scroll.AutoScrollMinSize+" display="+scroll.DisplayRectangle));}return String.Join(" | ",rows);}
 static async Task RevealPageControl(Control control,Panel pages){for(Control c=control.Parent;c!=null&&pages.Contains(c);c=c.Parent){var scroll=c as ScrollableControl;if(scroll!=null&&scroll.AutoScroll&&(scroll.VerticalScroll.Visible||scroll.HorizontalScroll.Visible)){scroll.ScrollControlIntoView(control);await Task.Delay(20);}}}
 internal static async Task Run(ExperimentShell shell,Form host,Func<Task> ready,Action<bool,string> check,Action<string,Form> image){
  await ready();string before=Context(shell),selected=Field<string>(shell,"selectedMacro");var windows=Field<Dictionary<string,Form>>(shell,"workWindows");
  Invoke(shell,"RunCommand","macro-search");await Task.Delay(50);check(windows.ContainsKey("solve")&&!windows.ContainsKey("macro")&&!windows.ContainsKey("operation"),"Macro Base opens the single canonical Solve window");
  var solve=windows["solve"];var originalBounds=solve.Bounds;var search=Field<TextBox>(shell,"search");string query=search.Text;var originalPage=Field<string>(shell,"solvePage");
  try{
   check(search.Focused&&Field<string>(shell,"solvePage")=="macros","F3 command route focuses Solve macro search without switching key set");
   Invoke(shell,"RunCommand","operation-focus");await Task.Delay(30);check(windows["solve"]==solve&&Field<FlowLayoutPanel>(shell,"operationStrip").ContainsFocus,"F4 command route focuses the same Solve execution region");
   var buttons=Field<Dictionary<Button,string>>(shell,"commandButtons");foreach(string action in new[]{"review","preview","commit","cancel-preview","cancel-analysis"})check(buttons.Any(pair=>pair.Value==action&&pair.Key.Parent==Field<FlowLayoutPanel>(shell,"operationStrip")),action+" retains its original guarded operation strip");
   solve.Size=solve.MinimumSize;await Task.Delay(60);
   foreach(string page in new[]{"macros","prepare","protection"}){
    Invoke(shell,"RunCommand","solve-"+page);await Task.Delay(30);check(Field<string>(shell,"solvePage")==page,"Explicit single-key-compatible route selects "+page+" without a hidden bank change");
    image("solve-"+page+"-minimum",solve);var execution=Field<FlowLayoutPanel>(shell,"operationStrip");Rectangle executionBounds=execution.RectangleToScreen(execution.ClientRectangle);
    foreach(var button in Descendants(solve).OfType<Button>().Where(button=>button.Visible&&button.Enabled&&buttons.ContainsKey(button)).ToArray()){if(!Exposed(button))await RevealPageControl(button,Field<Panel>(shell,"solvePages"));bool exposed=Exposed(button);if(!exposed)image("solve-"+page+"-failed-"+buttons[button],solve);check(exposed,"Minimum Solve exposes "+buttons[button]+" in "+page+(exposed?"":" | "+BoundsChain(button)));}
    check(executionBounds==execution.RectangleToScreen(execution.ClientRectangle)&&Exposed(execution),"Page scrolling keeps execution controls fixed and fully visible in "+page);
    if(page=="prepare")foreach(var role in Field<Dictionary<string,Button>>(shell,"solveRoles")){var b=role.Value;int needed=b.GetPreferredSize(new Size(b.Width,Int32.MaxValue)).Height;check(b.Height>=needed,"Role "+role.Key+" fits native mathematical position text layout: needs "+needed+", available "+b.Height);}
    if(page=="macros"){var library=buttons.Single(pair=>pair.Value=="macro-check-library"&&Field<TableLayoutPanel>(shell,"solveContent").Contains(pair.Key)).Key;check(Exposed(library),"Explicit Check library remains fully exposed beside selected-macro actions");}
    foreach(var readout in Descendants(solve).OfType<Label>().Where(label=>label.Parent.GetType().Name=="WorkReadout"))check(Exposed(readout),"Minimum Solve retains shared "+readout.Text.Split('\n')[0]);
    image("solve-"+page+"-checked",solve);
   }
   Invoke(shell,"RunCommand","solve-macros");search.Text="no-such-macro-identity-7c9a1f-empty-check";await Task.Delay(30);check(Field<ListBox>(shell,"macros").Items.Count==0,"Empty search has no invented default macro");check(Field<string>(shell,"selectedMacro")==selected,"Empty search retains inspected canonical macro identity");
   search.Text=query;await Task.Delay(30);check(Context(shell)==before,"Page changes, search and execution focus do not alter complete work context");
   Invoke(shell,"HideWorkWindow","operation");check(!solve.Visible&&!solve.IsDisposed,"Legacy operation close hides the reusable Solve window");Invoke(shell,"OpenWorkWindow","macro",true);check(windows["solve"]==solve&&solve.Visible,"Legacy Macro Base reopen reuses the same Solve instance");
  }finally{search.Text=query;solve.Bounds=originalBounds;Invoke(shell,"SelectSolvePage",originalPage,false);}
 }
}
