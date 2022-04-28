// // my implementing struct
// #[derive(Serialize, Deserialize)]
// struct Processor {
//     data: Option<OptionalData>
// }
//
// #[typetag::serde]
// impl EventProcessor for Processor {
//     fn process(&mut self, event_bus: &Vec<GameEvent>, max_events: usize, bus_tail: usize) { todo!() }
//
//     fn as_any_mut(&mut self) -> &mut dyn Any {self}
//     fn get_id(&self) -> &str { "my_processor" }
// }
//
// #[derive(Serialize, Deserialize)]
// struct OptionalData{} // some struct representing data specific to the processor
//
// // the game engine, which has a list of trait objects
// struct GameEngine {
//     event_processors: Vec<Box<dyn EventProcessor>>
// }
//
// impl GameEngine {
//     fn new() -> Self {
//         GameEngine{
//             event_processors: vec![Box::new(Processor{data: Some(OptionalData{}) })]
//         }
//     }
//
//     // function that will mutate some data on one of the processors
//     fn edit_data(&mut self) {
//         self.event_processors.iter_mut()
//             .find(|s| s.get_id() == "my_processor")
//             .map(|p| if let Some(processor) = p.as_any_mut().downcast_mut::<Processor>() {
//                 processor.data = Some(OptionalData{});
//             });
//     }
// }