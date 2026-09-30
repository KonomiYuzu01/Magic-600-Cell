using System;
using System.Collections.Generic;
using System.IO;

internal static class ExperimentSendOutcome {
 internal sealed class WorkerResult<T> {internal Dictionary<string,object> Reply,Receipt;internal T Next;internal Exception Fault;}
 internal sealed class FinalResult {internal Dictionary<string,object> LastResult;internal bool Accepted,LogRejection;internal string ReceiptWarning;}
 static object Value(Dictionary<string,object> value,string key){object result;return value!=null&&value.TryGetValue(key,out result)?result:null;}
 internal static WorkerResult<T> Worker<T>(Func<Dictionary<string,object>> post,Func<Dictionary<string,object>> refresh,Func<Dictionary<string,object>,bool,T> read,string requestedAction,Action<Exception> recoveryFailed){
  var outcome=new WorkerResult<T>();
  try{
   outcome.Reply=post();var primary=Value(outcome.Reply,"result") as Dictionary<string,object>;
   if(requestedAction=="session-log-apply"&&Object.Equals(Value(primary,"import_applied"),true)||Object.Equals(Value(primary,"committed"),true))outcome.Receipt=primary;
   if(Object.Equals(Value(outcome.Reply,"requires_refresh"),true)){var fresh=refresh();fresh["result"]=outcome.Reply["result"];outcome.Reply=fresh;outcome.Next=read(outcome.Reply,false);}
   else try{outcome.Next=read(outcome.Reply,true);}catch(InvalidDataException){var fresh=refresh();outcome.Next=read(fresh,false);fresh["result"]=outcome.Reply["result"];outcome.Reply=fresh;}
  }catch(Exception error){outcome.Fault=error;try{outcome.Reply=refresh();outcome.Next=read(outcome.Reply,false);}catch(Exception recovery){recoveryFailed(recovery);}}
  return outcome;
 }
 internal static FinalResult Finalize(Dictionary<string,object> receipt,bool isImport,Dictionary<string,object> primaryResult,Exception fault,bool connected,bool phaseComplete){
  string warning=null;
  if(receipt!=null&&(fault!=null||!connected)){
   string detail=fault==null?"Native display adoption failed.":fault.Message;
   if(isImport)warning="Log import was accepted. "+detail+(connected?" Current state was refreshed.":" Input remains disabled; reopen this session to recover its native display. Do not repeat the import.");
   else warning="Operation was committed. "+detail+(connected?" Current state was refreshed.":" Input remains disabled; reopen this session to recover its native display.")+" Do not repeat the operation.";
  }
  return new FinalResult{LastResult=receipt??(fault==null?primaryResult:null),Accepted=receipt!=null||fault==null&&connected&&phaseComplete,ReceiptWarning=warning,LogRejection=fault!=null&&receipt==null};
 }
}
