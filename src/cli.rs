use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Terminal client mirroring the FatSecret mobile app.
///
/// Same authorization, same requests: food search today; diary, recipes and
/// barcode return with their request-shape slices.
#[derive(Parser, Debug)]
#[command(
    name = "fatsecret-cli",
    version,
    about,
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    /// Increase log verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Config file (TOML): endpoints and device model
    #[arg(short, long, global = true, env = "FATSECRET_CONFIG")]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Human, global = true)]
    pub format: OutputFormat,

    /// Controls log color on stderr; data output stays uncolored
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto, global = true)]
    pub color: ColorChoice,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Log in with your FatSecret account (Google or password)
    Auth(AuthArgs),
    /// Search the food database
    Foods(FoodsArgs),
    /// Recipes and cookbook
    Recipes(RecipesArgs),
    /// Food diary
    Diary(DiaryArgs),
    /// Body weight
    Weight(WeightArgs),
    /// Daily calorie target (RDI)
    Rdi(RdiArgs),
    /// Exercise diary
    Exercise(ExerciseArgs),
    /// Water tracker (requires FatSecret Premium)
    Water(WaterArgs),
    /// Saved meals
    Meals(MealArgs),
    /// Multi-day meal plans
    MealPlans(MealPlansArgs),
    Account(AccountArgs),
    /// User settings and attributes
    Settings(SettingsArgs),
    /// Notifications
    Notifications(NotificationsArgs),
    /// Social feed blocks
    Feed(FeedArgs),
    /// Learning content and progress
    Learning(LearningArgs),
    /// Static food-groups data
    FoodGroups(FoodGroupsArgs),
    /// View and edit persistent CLI settings
    Config(ConfigArgs),
    /// Print shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show resolved settings (secrets masked)
    Show,
    /// Set a persistent setting (e.g. market_locale RU)
    Set {
        /// Setting key
        key: String,
        /// Setting value
        value: String,
    },
    /// Remove a persistent setting (falls back to default)
    Unset {
        /// Setting key
        key: String,
    },
    /// Print the settings file path
    Path,
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Sign in to an existing Google-linked account using Playwriter and Chrome
    Google {
        /// Read a Google ID token from stdin instead of opening the sign-in tab
        #[arg(long)]
        token_stdin: bool,
        /// Seconds to wait for interactive browser sign-in
        #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..=3600))]
        timeout: u64,
    },
    /// Log in as USERNAME (asks for the password interactively)
    Login {
        /// FatSecret username or email
        username: String,
    },
    /// Register a new account (asks for the password interactively)
    Register {
        /// Email address
        email: String,
        /// Desired member name
        username: String,
        /// Birth date YYYY-MM-DD
        #[arg(long)]
        birth_date: String,
        /// Gender as the app sends it (e.g. Male/Female)
        #[arg(long)]
        gender: String,
        /// Country code, e.g. US
        #[arg(long, default_value = "US")]
        country: String,
        /// Current weight in kg
        #[arg(long)]
        current_weight_kg: Option<f64>,
        /// Goal weight in kg
        #[arg(long)]
        goal_weight_kg: Option<f64>,
        /// Height in cm
        #[arg(long)]
        height_cm: Option<f64>,
        /// First name
        #[arg(long)]
        first_name: Option<String>,
    },
    /// Send a password-reset email
    ForgotPassword {
        /// Account email address
        email: String,
    },
    /// Complete a password reset (code + new password via prompts)
    ResetPassword,
    /// Show login state (username only, never secrets)
    Status,
    /// Delete stored credentials
    Logout,
}

#[derive(Args, Debug)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub action: AccountAction,
}

#[derive(Subcommand, Debug)]
pub enum AccountAction {
    /// Show account details
    Show,
    /// Show account settings page
    Settings,
    /// Change member name (takes effect immediately)
    ChangeUsername {
        /// New member name
        name: String,
    },
}

#[derive(Args, Debug)]
pub struct DiaryArgs {
    #[command(subcommand)]
    pub action: DiaryAction,
}

#[derive(Subcommand, Debug)]
pub enum DiaryAction {
    /// Show the diary day (default today); history supported
    Day {
        /// Date YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
    },
    /// Copy a day's entries onto another date (same meals, fresh ids)
    Cp {
        /// Source date YYYY-MM-DD
        #[arg(long)]
        from: String,
        /// Target date YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
    },
    /// Log a food entry
    Add {
        /// Food id from search
        #[arg(long)]
        food_id: i64,
        /// Serving/portion id from food details
        #[arg(long)]
        serving_id: i64,
        /// Meal: breakfast|lunch|dinner|snack|other (or --meal-id INT)
        #[arg(long, default_value = "other")]
        meal: String,
        /// Numeric meal id override
        #[arg(long)]
        meal_id: Option<i64>,
        /// Date YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
        /// Number of servings
        #[arg(long, default_value_t = 1.0)]
        units: f64,
    },
    /// Delete an entry by id (see it in `diary day`)
    Rm {
        /// Entry id
        entry_id: i64,
        /// Entry recorded date YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct WeightArgs {
    #[command(subcommand)]
    pub action: WeightAction,
}

#[derive(Subcommand, Debug)]
pub enum WeightAction {
    /// Log body weight in kg (goal defaults to current account goal)
    Log {
        /// Weight in kilograms
        kg: f64,
        /// Goal weight in kg (default: account goal)
        #[arg(long)]
        goal_kg: Option<f64>,
    },
}

#[derive(Args, Debug)]
pub struct RdiArgs {
    #[command(subcommand)]
    pub action: RdiAction,
}

#[derive(Subcommand, Debug)]
pub enum RdiAction {
    /// Show the current daily calorie target
    Show,
    /// Save the daily calorie target
    Save {
        /// Age in years
        #[arg(long)]
        age: i64,
        /// Weight in kilograms
        #[arg(long)]
        weight_kg: f64,
        /// Height in centimeters
        #[arg(long)]
        height_cm: f64,
        /// Sex: female|male
        #[arg(long)]
        sex: String,
        /// Goal: maintain|gain|gain-slow|lose-slow|lose
        #[arg(long)]
        goal: String,
        /// Activity: sedentary|low|active|high
        #[arg(long)]
        activity: String,
        /// Target calories per day
        #[arg(long)]
        rdi: i64,
    },
}

#[derive(Args, Debug)]
pub struct ExerciseArgs {
    #[command(subcommand)]
    pub action: ExerciseAction,
}

#[derive(Subcommand, Debug)]
pub enum ExerciseAction {
    /// Show exercise entries for a day (default today)
    Day {
        /// Date YYYY-MM-DD (default today)
        #[arg(long)]
        date: Option<String>,
    },
    /// List activity types
    Types,
    /// Log an activity
    Log {
        /// Activity type id (see `exercise types`)
        #[arg(long)]
        type_id: i64,
        /// Minutes performed
        #[arg(long)]
        mins: i64,
        /// Calories expended (optional, server estimates otherwise)
        #[arg(long)]
        kcal: Option<f64>,
        /// Free-text note
        #[arg(long)]
        description: Option<String>,
    },
}

#[derive(Args, Debug)]
pub struct WaterArgs {
    #[command(subcommand)]
    pub action: WaterAction,
}

#[derive(Subcommand, Debug)]
pub enum WaterAction {
    /// Show today's water entry (requires FatSecret Premium)
    Day,
    /// Log water intake in ml (requires FatSecret Premium)
    Log {
        /// Milliliters consumed
        ml: i64,
        /// Daily goal in ml (default 2000)
        #[arg(long, default_value_t = 2000)]
        goal_ml: i64,
    },
}

#[derive(Args, Debug)]
pub struct MealArgs {
    #[command(subcommand)]
    pub action: MealAction,
}

#[derive(Subcommand, Debug)]
pub enum MealAction {
    /// List saved meals
    Ls {
        /// Meal-tab ordinal filter
        #[arg(long, default_value_t = 0)]
        meal: i64,
    },
    /// Show one saved meal with items
    Show {
        /// Saved meal id
        id: i64,
    },
    /// Create a saved meal (mealid 0)
    Create {
        /// Title
        #[arg(long)]
        title: String,
        /// Description
        #[arg(long, default_value = "")]
        description: String,
        /// Meal types mask
        #[arg(long, default_value = "1")]
        meal_types: String,
    },
    /// Edit a saved meal
    Save {
        /// Saved meal id
        id: i64,
        /// Title
        #[arg(long)]
        title: String,
        /// Description
        #[arg(long, default_value = "")]
        description: String,
        /// Meal types mask
        #[arg(long, default_value = "1")]
        meal_types: String,
    },
    /// Delete a saved meal
    Rm {
        /// Saved meal id
        id: i64,
    },
    /// Log a saved meal into the diary
    Log {
        /// Saved meal id
        id: i64,
        /// Meal: breakfast|lunch|dinner|snack|other (or --meal-id INT)
        #[arg(long, default_value = "other")]
        meal: String,
        /// Numeric meal id override
        #[arg(long)]
        meal_id: Option<i64>,
    },
    /// Add (or edit, with --item-id) a food item in a saved meal
    AddItem {
        /// Saved meal id
        #[arg(long)]
        meal_id: i64,
        /// Food id from search
        #[arg(long)]
        food_id: i64,
        /// Entry name
        #[arg(long)]
        name: String,
        /// Portion id from food details
        #[arg(long)]
        portion_id: i64,
        /// Number of servings
        #[arg(long, default_value_t = 1.0)]
        units: f64,
        /// Existing item id (edit instead of add)
        #[arg(long, default_value_t = 0)]
        item_id: i64,
    },
    /// Delete a food item from a saved meal
    RmItem {
        /// Item id
        item_id: i64,
    },
    /// Quick picks list
    Quickpicks,
    /// Duplicate a saved meal under a new title
    Duplicate {
        /// Source saved meal id
        #[arg(long)]
        id: i64,
        /// Title for the copy
        #[arg(long)]
        title: String,
        /// Description for the copy
        #[arg(long, default_value = "")]
        description: String,
        /// Meal types mask
        #[arg(long, default_value = "1")]
        meal_types: String,
    },
}

#[derive(Args, Debug)]
pub struct MealPlansArgs {
    #[command(subcommand)]
    pub action: MealPlansAction,
}

#[derive(Subcommand, Debug)]
pub enum MealPlansAction {
    /// Save a meal plan (create with --plan-id 0)
    Save {
        /// Plan id (0 to create)
        #[arg(long, default_value_t = 0)]
        plan_id: i64,
        /// Plan name
        #[arg(long, default_value = "")]
        name: String,
        /// Plan description
        #[arg(long, default_value = "")]
        description: String,
        /// Entry as DAY:RECIPE:PORTION:AMOUNT:MEAL:NAME, repeatable
        #[arg(long = "entry", value_name = "DAY:RECIPE:PORTION:AMOUNT:MEAL:NAME")]
        entries: Vec<String>,
    },
    /// Schedule plan weeks (insert/delete PLAN:DAYINT pairs)
    Schedule {
        /// Insert as PLAN:DAYINT, repeatable
        #[arg(long = "insert", value_name = "PLAN:DAYINT")]
        inserts: Vec<String>,
        /// Delete as PLAN:DAYINT, repeatable
        #[arg(long = "delete", value_name = "PLAN:DAYINT")]
        deletes: Vec<String>,
    },
}

#[derive(Args, Debug)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub action: SettingsAction,
}

#[derive(Subcommand, Debug)]
pub enum SettingsAction {
    /// User attributes (birth date, gender, names)
    Attributes,
    /// Locale configuration (language id)
    Locale,
}

#[derive(Args, Debug)]
pub struct NotificationsArgs {
    #[command(subcommand)]
    pub action: NotificationsAction,
}

#[derive(Subcommand, Debug)]
pub enum NotificationsAction {
    /// List notifications
    Ls,
}

#[derive(Args, Debug)]
pub struct FeedArgs {
    #[command(subcommand)]
    pub action: FeedAction,
}

#[derive(Subcommand, Debug)]
pub enum FeedAction {
    /// Users blocking you
    Blocks,
    /// Users you block
    Blocking,
    /// Block a user by id (no unblock: the app has no wired call)
    Block {
        /// User id to block
        user_id: i64,
    },
}

#[derive(Args, Debug)]
pub struct LearningArgs {
    #[command(subcommand)]
    pub action: LearningAction,
}

#[derive(Subcommand, Debug)]
pub enum LearningAction {
    /// Learning progress and bookmarks
    Progress,
    /// Learning content for your language
    Content,
    /// Bookmark a guided course by content id
    BookmarkCourse {
        /// Guided course content id
        id: i64,
    },
    /// Remove a guided course bookmark
    UnbookmarkCourse {
        /// Guided course content id
        id: i64,
    },
    /// Bookmark a lesson by content id
    BookmarkLesson {
        /// Lesson content id
        id: i64,
    },
    /// Remove a lesson bookmark
    UnbookmarkLesson {
        /// Lesson content id
        id: i64,
    },
    /// Mark a lesson complete
    CompleteLesson {
        /// Lesson content id
        id: i64,
    },
}

#[derive(Args, Debug)]
pub struct FoodGroupsArgs {}

#[derive(Args, Debug)]
pub struct FoodsArgs {
    #[command(subcommand)]
    pub action: FoodsAction,
}

#[derive(Args, Debug)]
pub struct RecipesArgs {
    #[command(subcommand)]
    pub action: RecipesAction,
}

#[derive(Subcommand, Debug)]
pub enum RecipesAction {
    /// Search recipes (legacy cookbook index)
    Search {
        /// Search expression
        query: String,
        /// Zero-based page number
        #[arg(long, default_value_t = 0)]
        page: i64,
    },
    /// Full details for a recipe id (same page as food details)
    Get {
        /// Recipe id
        id: i64,
    },
    /// List recipe categories
    Categories,
    /// Search the cookbook index
    CookbookSearch {
        /// Search expression
        query: String,
        /// Zero-based page number
        #[arg(long, default_value_t = 0)]
        page: i64,
        /// Results per page
        #[arg(long, default_value_t = 20)]
        size: i64,
    },
    /// Cookbook recipe count per market
    CookbookCount {
        /// Market locale, e.g. US
        #[arg(long, default_value = "US")]
        market: String,
    },
    /// Create a recipe (returns the new id)
    Create {
        /// Title
        #[arg(long)]
        title: String,
        /// Description
        #[arg(long, default_value = "")]
        description: String,
        /// Servings count
        #[arg(long, default_value_t = 1.0)]
        portions: f64,
        /// Prep time in minutes
        #[arg(long, default_value_t = 0)]
        prep_time: i64,
        /// Cook time in minutes
        #[arg(long, default_value_t = 0)]
        cook_time: i64,
    },
    /// Edit a recipe (steps/types replace server state)
    Save {
        /// Recipe id
        id: i64,
        /// Title
        #[arg(long)]
        title: String,
        /// Description
        #[arg(long, default_value = "")]
        description: String,
        /// Servings count
        #[arg(long, default_value_t = 1.0)]
        portions: f64,
        /// Prep time in minutes
        #[arg(long, default_value_t = 0)]
        prep_time: i64,
        /// Cook time in minutes
        #[arg(long, default_value_t = 0)]
        cook_time: i64,
        /// Share publicly (default private)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        share: bool,
        /// Step text, repeatable in order (step1, step2, ...)
        #[arg(long = "step", value_name = "TEXT")]
        steps: Vec<String>,
        /// Recipe type id, repeatable
        #[arg(long = "type", value_name = "ID")]
        types: Vec<i64>,
    },
    /// Delete a recipe
    Rm {
        /// Recipe id
        id: i64,
    },
    /// Add (or edit, with --item-id) an ingredient in a recipe
    AddIngredient {
        /// Parent recipe id
        #[arg(long)]
        recipe_id: i64,
        /// Food id from search
        #[arg(long)]
        food_id: i64,
        /// Entry name
        #[arg(long)]
        name: String,
        /// Portion id from food details
        #[arg(long)]
        portion_id: i64,
        /// Number of servings
        #[arg(long, default_value_t = 1.0)]
        units: f64,
        /// Existing ingredient id (edit instead of add)
        #[arg(long, default_value_t = 0)]
        item_id: i64,
    },
    /// Delete an ingredient from a recipe
    RmIngredient {
        /// Ingredient id
        #[arg(long)]
        item_id: i64,
        /// Parent recipe id
        #[arg(long)]
        recipe_id: i64,
    },
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum FoodsAction {
    /// Search the food database
    Search {
        /// Search expression, e.g. "oatmeal"
        query: String,
        /// Zero-based page number
        #[arg(long, default_value_t = 0)]
        page: i64,
        /// Results per page
        #[arg(long, default_value_t = 20)]
        size: i64,
        /// Market index override (e.g. RU); default from config
        #[arg(long)]
        market: Option<String>,
        /// Language override (e.g. ru); default from config
        #[arg(long)]
        lang: Option<String>,
    },
    /// Full details for a food id (portions included when served)
    Get {
        /// Food id from search
        id: i64,
    },
    /// Most popular serving sizes for one or more food ids
    Popular {
        /// Food ids
        #[arg(required = true)]
        ids: Vec<i64>,
    },
    /// Vote on a dietary preference: repeat --vote TYPE=VALUE
    VotePreference {
        /// Food id
        id: i64,
        /// Vote as TYPE=VALUE, repeatable
        #[arg(long = "vote", value_name = "TYPE=VALUE")]
        votes: Vec<String>,
    },
    /// Scan a barcode (GTIN) and show the food
    Barcode {
        /// GTIN digits
        gtin: String,
    },
    /// List dietary-preference/allergen types for votes
    PreferenceTypes,
    /// Create a custom food (own recipe entry)
    Create {
        /// Product name
        #[arg(long)]
        name: String,
        /// Serving size text (e.g. "1 cup")
        #[arg(long, default_value = "")]
        serving_size: String,
        /// Metric serving (e.g. "100g")
        #[arg(long, default_value = "")]
        metric_serving: String,
        /// Brand / manufacturer name
        #[arg(long, default_value = "")]
        brand: String,
        /// Manufacturer type ordinal (0 Own, 1 Manufacturer, 2 Restaurant, 3 Supermarket, 4 Brewer, 5 Other)
        #[arg(long, default_value_t = 0)]
        manufacturer_type: i64,
        /// Pipe-separated tags
        #[arg(long, default_value = "")]
        tags: String,
        /// Salt-based serving
        #[arg(long, action = clap::ArgAction::SetTrue)]
        is_salt: bool,
        /// Barcode digits (optional)
        #[arg(long, default_value = "")]
        barcode: String,
        /// Barcode type name (UPC_A|UPC_E|EAN_8|EAN_13|Other)
        #[arg(long, default_value = "EAN_13")]
        barcode_type: String,
        /// Calories per serving
        #[arg(long)]
        calories: Option<f64>,
        /// Protein grams per serving
        #[arg(long)]
        protein: Option<f64>,
        /// Carbohydrate grams per serving
        #[arg(long)]
        carbs: Option<f64>,
        /// Fat grams per serving
        #[arg(long)]
        fat: Option<f64>,
        /// Fiber grams per serving
        #[arg(long)]
        fiber: Option<f64>,
        /// Sugar grams per serving
        #[arg(long)]
        sugar: Option<f64>,
        /// Sodium milligrams per serving
        #[arg(long)]
        sodium: Option<f64>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum OutputFormat {
    /// Pretty tables for humans
    #[default]
    Human,
    /// JSON to stdout (pipe-safe)
    Json,
    /// Minimal tab-separated output for scripts
    Plain,
}

#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}
