// Pascal / Delphi — TList of records, sorted by priority.

program TaskList;

{$APPTYPE CONSOLE}

uses
  SysUtils, Classes, Generics.Collections, Generics.Defaults;

type
  TTask = record
    Title: string;
    Priority: Integer;
  end;

  TTaskList = TList<TTask>;

procedure FillBacklog(out list: TTaskList);
begin
  list := TTaskList.Create;
  list.Add(TTask.Create('Write tests', 2));
  list.Add(TTask.Create('Fix login bug', 5));
  list.Add(TTask.Create('Refactor parser', 3));
end;

procedure PrintAll(list: TTaskList);
var
  t: TTask;
begin
  for t in list do
    Writeln(Format('%d  %s', [t.Priority, t.Title]));
end;

var
  backlog: TTaskList;
begin
  FillBacklog(backlog);
  try
    backlog.Sort(TComparer<TTask>.Construct(
      function(const L, R: TTask): Integer
      begin
        Result := L.Priority - R.Priority;
      end));
    PrintAll(backlog);
  finally
    backlog.Free;
  end;
end.