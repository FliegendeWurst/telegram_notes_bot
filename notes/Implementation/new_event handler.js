function formatTime(startTime) {
    return startTime.getFullYear().toString() + "-" + (startTime.getMonth() + 1).toString().padStart(2, "0") + "-" + startTime.getDate().toString().padStart(2, "0") + "T" + startTime.getHours().toString().padStart(2, "0") + ":" + startTime.getMinutes().toString().padStart(2, "0") + ":" + startTime.getSeconds().toString().padStart(2, "0");
}

const {req, res} = api;

const uid = req.body["uid"];
const name = req.body["name"];
const summary = req.body["summary"];
const summaryHtml = req.body["summaryHtml"];
const location = req.body["location"];
const fileName = req.body["fileName"];
const fileData = req.body["fileData"];
const startTime = new Date(req.body["startTime"]);
const endTime = new Date(req.body["endTime"]);

const year = startTime.getFullYear();
var month = startTime.getMonth() + 1;
var day = startTime.getDate();

if (month < 10) {
  month = '0' + month;
}
if (day < 10) {
  day = '0' + day;
}

const eventDateStr = year + "-" + month + "-" + day;
const dayNote = await api.getDateNote(eventDateStr);

const targetTemplate = await api.currentNote.getRelationValue('targetTemplate');
const options = {
    "parentNoteId": dayNote.noteId,
    "title": name,
    "content": summaryHtml != "" ? summaryHtml : summary,
    "type": "text",
    "mime": summaryHtml != "" ? "text/html" : "text/plain"
};
const resp = await api.createNewNote(options);
const note = resp.note;
await note.setAttribute("relation", "template", targetTemplate);
await note.setAttribute("label", "uid", uid);
await note.setAttribute("label", "location", location);
const startTimeStr = formatTime(startTime);
await note.setAttribute("label", "startTime", startTimeStr);
await note.setAttribute("label", "endTime", formatTime(endTime));
const fileOptions = {
    "parentNoteId": note.noteId,
    "title": fileName,
    "content": fileData,
    "type": "file",
    "mime": "text/calendar"
};
await api.createNewNote(fileOptions);

res.sendStatus(200);
