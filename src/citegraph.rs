// TODO: track the edges in the citation graph structure, make a dictionary of ids to indeces into the nodearray?

// Constructs a Directed Acyclic Graph of Academic Papers by their citation relationships

use std::{io, string};
// Can Be used with Wasm
use scraper::{Html, Selector};
use regex::Regex;

const DEBUG:bool = true;

#[derive(Debug, Clone)]
pub struct PaperID {
    value:[u8; 12]
}
impl PaperID {
    fn new_from_str(str:&str) -> Result<Self, &str> {
        match str.len() {
            12 => {
                return Ok( PaperID { value: str.as_bytes().try_into().expect("could not convert string to u8 array of size 12") } )
            },
            _ => Err("String does not match the format of a Google Scholar ID")
        }
    }
}
impl std::fmt::Display for PaperID {
    fn fmt(self:&Self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!( f, "{}", self.value.iter().map(|num| *num as char).collect::<String>() )
    }
}

// stores the details of a google scholar paper
#[derive(Debug, Clone)]
pub struct PaperNode {
    //authors:Vec<String>,  // stores pairing of authors to the links to their google scholar page
    cited_by_url:Option<String>,  // url that leads to resource that cites this paper
    cited_by:Vec<PaperID>, // vector of ids of the papers that cite this one
    id:PaperID,            // unique id of this Paper Node
    //source_url:String,    // link to the actual paper
    title_abbr:String,    // abbreviated title of the paper
    year:u32,             // year published
}
impl PaperNode {
    // finds the first subsection of the supplied html that contains info about a paper and parses it
    fn get_title(self:&Self) -> String {
        self.title_abbr.clone()
    }
}

// takes a string and returns an integer that could be a year a paper on google scholar was published
fn find_year_in_string(text_with_year: &str) -> Option<u32> {
    // find all numbers in string
    let re = Regex::new(r"(-?\d+)").unwrap();
    for num_text in re.captures_iter(text_with_year) {
        let num_result = num_text.get(0).unwrap().as_str().parse::<i32>();
        match num_result {
            Ok(number) if number > 1000 => { return Some(number as u32); },
            Err(_) | Ok(_) => {}
        }
    }
    None
}

fn remove_html_tags(str_with_tags:&str) -> String {
    let re_remove_tags:Regex = Regex::new(r"<[^>]*>").unwrap();
    return re_remove_tags.replace_all(&str_with_tags, "").to_string();
}

fn div_frag_into_node(div_frag_str:&str) -> Option<PaperNode> {
    let parsed_html:Html = Html::parse_fragment(div_frag_str);
    let paper_div_selector:Selector = Selector::parse("div.gs_ri").unwrap();
    let paper_div_list = parsed_html.select(&paper_div_selector);
    
    for paper in paper_div_list { // iterates through vector
        // get the section with the publication info
        // TODO: extract the authors add in author links
        let pub_div_selector = Selector::parse("div.gs_a").unwrap();
        let mut pub_sections = paper.select(&pub_div_selector);
        let pub_section_with_tags = match pub_sections.next() {
            Some(x) => { x.inner_html() },
            None => { "".to_string() }
        };
        let pub_string = remove_html_tags(&pub_section_with_tags);
        // find a publication year in the string, else set to zero
        let pub_year:u32 = match find_year_in_string(&pub_string) {
            Some(year) => year,
            None => 0
        };

        // extract the title
        let h3a_selector = Selector::parse("h3.gs_rt>a").unwrap();
        let mut h3a_sections = paper.select(&h3a_selector);
        let title_section = h3a_sections.next();
        let title_with_tags = match title_section {
            Some(x) => { x.inner_html() },
            None => { "".to_string() }
        };
        let title:String = remove_html_tags(&title_with_tags);

        // extract unique id
        let id_string:String = match title_section {
            Some(section) => { 
                match section.value().id() {
                    Some(id_string) => id_string.to_string(),
                    None => "".to_string()
                }
            },
            None => { "".to_string() }
        };
        let id:PaperID = match PaperID::new_from_str(&id_string) {
            Ok(paper_id) => paper_id,
            Err(_) => { 
                eprintln!("Failed to find ID for paper with title {title}");
                return None;
            }
        };

        // extract cited by URL
        let a_selector = Selector::parse("div.gs_fl>a").unwrap();
        let a_sections = paper.select(&a_selector);
        let mut cited_by_link:Option<String> = None;
        for a_section in a_sections {
            match a_section.value().attr("href") {
                Some(href) if href.to_string().starts_with("/scholar?cites=") => {
                    cited_by_link = Some( format!("https://scholar.google.com/{}", href.to_string()) );
                    break;
                },
                None | Some(_) => {}
            }
        }

        let title:String = remove_html_tags(&title_with_tags);
        
        if DEBUG {
            println!("\n\n{}", title);
            println!("Year: {}", pub_year);
            println!("ID: {:?}", id);
            match cited_by_link {
                Some(_) => {
                    println!("cited by url:{}", cited_by_link.clone().unwrap());
                },
                None => { println!("cited by url: *void*")}
            }
        }

        return Some(PaperNode {
            cited_by_url:cited_by_link,
            cited_by:Vec::new(), // create empty list of ids as a placeholder
            id:id,
            title_abbr:title,
            year:pub_year
        })

    }
    None
}

// returns an iterator over paper divs
pub fn parse_page_into_paper_nodes(html:&str) -> Vec<PaperNode> {
    let mut new_node_list:Vec<PaperNode>  = Vec::new();
    let parsed_html = Html::parse_fragment(&html);
    let paper_div_selector = Selector::parse("div.gs_ri").unwrap();
    for paper_div_fragment in parsed_html.select(&paper_div_selector){
        match div_frag_into_node(&paper_div_fragment.html()) {
            Some(new_node) => {
                // add node to the graph
                new_node_list.push(new_node);
            }
            None => {}
        }
    }
    new_node_list
}

// stores the state of the explored citation graph
#[derive(Debug)]
pub struct CitationGraph {
    level_populations:Vec<u32>, // record the number of nodes at each concentric circle around the center
    paper_nodes:Vec<PaperNode>,
    // TODO: hashmap of node IDs to indeces in the node array to make lookups faster
}
impl CitationGraph {
    pub fn new() -> Self {
        CitationGraph { level_populations: Vec::new(), paper_nodes: Vec::new() }
    } 
    fn add_page_to_graph(self:&mut Self, html:&str) {
        // parse this page into the paper divs
        let mut new_nodes = parse_page_into_paper_nodes(html);
        self.paper_nodes.append(&mut new_nodes);
        // TODO: update level popultaions
    }
    fn init_graph_from_page(html:&str) -> Self {
        let mut new_graph:CitationGraph = CitationGraph { level_populations: Vec::new(), paper_nodes: Vec::new() };
        new_graph.add_page_to_graph(html);
        // TODO: update level popultaions
        new_graph
    }
    fn expand_node(self:&mut Self, index:usize, cited_by_html:&str) {
        if index >= self.paper_nodes.len() {
            eprintln!("index does not correspond with a paper in this graph\n");
        }
        let parsed_nodes = parse_page_into_paper_nodes(cited_by_html);
        // iterate through the nodes adding each one to the graph and adding the ids to the citers of the paper node
        for node in parsed_nodes {
            self.paper_nodes[index].cited_by.push(node.id.clone());
            self.paper_nodes.push(node);
        }
    }
    fn add_paper_node(self:&mut Self, node:PaperNode) {
        self.paper_nodes.push(node);
    }
}
impl std::fmt::Display for CitationGraph {
    fn fmt(self:&Self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Citation Graph {{\n");
        let mut index:usize = 0;
        for node in &self.paper_nodes {
            write!(f, "    {index} -----\n");
            write!(f, "      Title: {}\n", node.get_title());
            write!(f, "      ID: {}\n", node.id);  
            match node.cited_by.len() {
                0 => {},
                _ => {
                    write!(f,"      Cited by: ");
                    for id in node.cited_by.iter() {
                        print!("            {}\n", id)
                    }
                }
            }
            write!(f, "      Can be expanded: {}\n", node.cited_by_url != None);
            index += 1;
        }
        write!(f, "}}")
    }
}

fn construct_gs_search_url(search_term:&str) -> String {
    // replace spaces with "+" character
    let formatted_terms = search_term.replace(" ", "+");
    format!("https://scholar.google.com/scholar?hl=en&as_sdt=0%2C43&q={formatted_terms}&btnG=")
}

//#[test]
// fn interactive_explore() {
//     // ask for search term
//     let mut search_term = String::new();
//     loop {
//         println!("\n\nEnter your search term: ");
//         let result = io::stdin().read_line(&mut search_term);
//         match result {
//             Ok(_) => break,
//             Err(_) => { print!("\nFailed to read input, please try again: ") }
//         }
//     }
    
//     // construct google scholar search url from search term
//     let url = construct_gs_search_url(&search_term);

//     // parse the search results page into PaperNodes
//     let nodes_on_search_page = standalone_parse_page(&url);

//     //
//     let mut cite_graph:CitationGraph = CitationGraph::new();
//     match nodes_on_search_page {
//         Some(nodelist) => {
//             // print the titles of all the nodes and ask which should be the center of the citation graph
//             println!("select a paper from the search results that you'd like to be the center of your graph: ");
//             for index in 0..nodelist.len() {
//                 println!("{index}: {}", nodelist[index].get_title());
//             }
//             let mut paper_selection:String = String::new();
//             let result = io::stdin().read_line(&mut paper_selection);
//             match paper_selection.trim_end_matches('\n').parse::<usize>() {
//                 Ok(s) if s < nodelist.len() => {
//                     // add the node to the graph
//                     cite_graph.add_paper_node(nodelist[s].clone());
//                 },
//                 Ok(_) => eprintln!("Selection not a valid index into search results array"),
//                 Err(err) => { eprintln!("invalid selection: {paper_selection}, error: {err}") }
//             }
//         },
//         None => { 
//             println!("\nNo nodes found in the page representing the search results");
//             return;
//         }
//     }

    
    
//     // loop until the user want to stop expanding nodes in the graph
//     loop {
//         println!("Citation Graph State: ");
//         println!("{}\n\n", cite_graph);
        
//         println!("Enter the index of a node in the graph you'd like to expand {cite_graph}");
//         let mut paper_selection:String = String::new();
//         let result = io::stdin().read_line(&mut paper_selection);
//         match paper_selection.trim_end_matches('\n').parse::<usize>() {
//             Ok(index) if index < cite_graph.paper_nodes.len() => {
//                 // add the node to the graph
//                 cite_graph.expand_node_standalone(index);
//                 println!("Node {index} expanded")
//             },
//             Ok(_) => eprintln!("Selection not a valid index into search results array"),
//             Err(err) => { eprintln!("invalid selection: {paper_selection}, error: {err}") }
//         }
//     }
    
// }