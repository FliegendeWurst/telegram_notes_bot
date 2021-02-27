// Create as JS Backend note and add these attributes:
// ~targetTemplateEvent=@event template (this note is described below)
// #customRequestHandler=event_alerts
// Create another note (event template) with this promoted attributes:
// startTime
// Optionally add:
// endTime, location

const {req, res} = api;

const targetTemplate = await api.currentNote.getRelationValue("targetTemplateEvent");
const events = await api.getNotesWithLabel("template", targetTemplate);

let eventsData = [];

for (const event of events) {
    const attr = await event.getAttribute("label", "startTime");
    eventsData.push({
        name: event.title,
        startTime: attr.value
    });
}

res.send(eventsData);
