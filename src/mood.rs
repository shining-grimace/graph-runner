use crate::{markers::Player, state::AppState};
use bevy::{color::palettes::css, prelude::*};

const MOOD_TRANSITION_DURATION: f32 = 2.0;

#[derive(Default, Copy, Clone)]
pub enum Mood {
    #[default]
    Peace,
    CallToAdventure,
    Confidence,
    TwistNewSubquest,
    SubquestTension,
    ReliefAfterSubquest,
    TwistUnconstrainedQuest,
    Triumph,
    ReliefAfterQuest,
}

impl Mood {
    pub fn get_background_color(&self) -> Color {
        match self {
            Mood::Peace => ClearColor::default().0,
            Mood::CallToAdventure => css::RED.into(),
            Mood::Confidence => css::DARK_GRAY.into(),
            Mood::TwistNewSubquest => css::BLUE_VIOLET.into(),
            Mood::SubquestTension => css::OLIVE.into(),
            _ => css::ALICE_BLUE.into(),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum PhysicalLevelSegment {
    #[default]
    Freedom,
    ConfidentQuest,
    Subquest,
    FinishQuest,
    FinalRelief,
}

impl PhysicalLevelSegment {
    fn get_starting_x(&self) -> f32 {
        match self {
            Self::Freedom => -1000.0,
            Self::ConfidentQuest => 5.0,
            Self::Subquest => 10.0,
            Self::FinishQuest => 15.0,
            Self::FinalRelief => 20.0,
        }
    }

    pub fn for_x(x: f32) -> Self {
        let all_cases = [
            Self::Freedom,
            Self::ConfidentQuest,
            Self::Subquest,
            Self::FinishQuest,
            Self::FinalRelief,
        ];
        for case in all_cases.iter().rev() {
            let threshold = case.get_starting_x();
            if x > threshold {
                return *case;
            }
        }
        Self::FinalRelief
    }

    pub fn get_mood(&self) -> Mood {
        match self {
            Self::Freedom => Mood::Peace,
            Self::ConfidentQuest => Mood::Confidence,
            Self::Subquest => Mood::SubquestTension,
            Self::FinishQuest => Mood::Triumph,
            Self::FinalRelief => Mood::ReliefAfterQuest,
        }
    }
}

#[derive(Message)]
pub struct NewMood(Mood);

#[derive(Default, Resource)]
struct MoodParams {
    pub current_mood: Mood,
    pub transitioning_from_color: Option<Color>,
    pub transition_progress: f32,
    pub current_physical_segment: PhysicalLevelSegment,
}

impl MoodParams {
    pub fn get_current_color(&self) -> Color {
        let Some(transitioning_from_color) = self.transitioning_from_color else {
            return self.current_mood.get_background_color();
        };
        let next_color = self.current_mood.get_background_color();
        return transitioning_from_color.mix(
            &next_color,
            self.transition_progress / MOOD_TRANSITION_DURATION,
        );
    }
}

pub struct MoodPlugin;

impl Plugin for MoodPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NewMood>()
            .insert_resource(MoodParams::default())
            .add_systems(
                PreUpdate,
                (
                    process_mood_events,
                    process_player_position,
                    transition_moods,
                )
                    .chain()
                    .run_if(in_state(AppState::Game)),
            );
    }
}

fn process_mood_events(mut mood_params: ResMut<MoodParams>, mut events: MessageReader<NewMood>) {
    for event in events.read() {
        mood_params.transitioning_from_color = Some(mood_params.get_current_color());
        mood_params.current_mood = event.0;
        mood_params.transition_progress = 0.0;
    }
}

fn process_player_position(
    player_query: Query<&Transform, With<Player>>,
    mut mood_params: ResMut<MoodParams>,
) -> Result<(), BevyError> {
    let player_transform = player_query.single()?;
    let segment = PhysicalLevelSegment::for_x(player_transform.translation.x);
    if segment != mood_params.current_physical_segment {
        mood_params.current_physical_segment = segment;
        mood_params.transitioning_from_color = Some(mood_params.get_current_color());
        mood_params.current_mood = segment.get_mood();
        mood_params.transition_progress = 0.0;
    }
    Ok(())
}

fn transition_moods(
    mut mood_params: ResMut<MoodParams>,
    mut clear_color: ResMut<ClearColor>,
    time: Res<Time>,
) {
    if mood_params.transitioning_from_color.is_none() {
        return;
    }
    mood_params.transition_progress = mood_params.transition_progress + time.delta_secs();
    if mood_params.transition_progress >= MOOD_TRANSITION_DURATION {
        mood_params.transition_progress = MOOD_TRANSITION_DURATION;
        mood_params.transitioning_from_color = None;
    }
    clear_color.0 = mood_params.get_current_color();
}
